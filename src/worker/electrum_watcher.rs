// MyCitadel desktop wallet: bitcoin & RGB wallet based on GTK framework.
//
// Written in 2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoraprime.ch>
//
// Copyright (C) 2022 by Pandora Prime Sarl, Switzerland.
//
// This software is distributed without any warranty. You should have received
// a copy of the AGPL-3.0 License along with this software. If not, see
// <https://www.gnu.org/licenses/agpl-3.0-standalone.html>.

use std::collections::BTreeMap;
use std::thread::JoinHandle;
use std::time::Duration;
use std::{io, thread};

use amplify::Wrapper;
use bitcoin::{Transaction, Txid};
use electrum_client::{Client as ElectrumClient, ElectrumApi, HeaderNotification};
use relm::Sender;
use wallet::address::address::AddressCompat;
use wallet::hd::{SegmentIndexes, UnhardenedIndex};
use wallet::scripts::PubkeyScript;

use crate::model::WalletSettings;

pub enum WatchMsg {
    Connecting,
    Connected,
    Complete,
    LastBlock(HeaderNotification),
    LastBlockUpdate(HeaderNotification),
    FeeEstimate(f64, f64, f64),
    HistoryBatch(Vec<HistoryTxid>, u16),
    UtxoBatch(Vec<UtxoTxid>, u16),
    TxBatch(BTreeMap<Txid, Transaction>, f32),
    Error(electrum_client::Error),
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[derive(StrictEncode, StrictDecode)]
pub struct HistoryTxid {
    pub txid: Txid,
    pub height: i32,
    pub address: AddressCompat,
    pub index: UnhardenedIndex,
    pub change: bool,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[derive(StrictEncode, StrictDecode)]
pub struct UtxoTxid {
    pub txid: Txid,
    pub height: u32,
    pub pos: u32,
    pub value: u64,
    pub address: AddressCompat,
    pub index: UnhardenedIndex,
    pub change: bool,
}

pub struct ElectrumWatcher {
    handle: JoinHandle<()>,
}

impl ElectrumWatcher {
    pub fn with(
        sender: Sender<WatchMsg>,
        wallet_settings: WalletSettings,
    ) -> Result<Self, io::Error> {
        Ok(Self {
            handle: std::thread::Builder::new()
                .name(s!("electrum-watcher"))
                .spawn(move || {
                    let err = electrum_watcher(&sender, wallet_settings).unwrap_err();
                    sender.send(WatchMsg::Error(err)).expect("channel broken");
                })?,
        })
    }
}

pub fn electrum_watcher(
    sender: &Sender<WatchMsg>,
    wallet_settings: WalletSettings,
) -> Result<(), electrum_client::Error> {
    sender
        .send(WatchMsg::Connecting)
        .expect("electrum watcher channel is broken");

    let config = electrum_client::ConfigBuilder::new()
        .timeout(Some(5))
        .expect("we do not use socks here")
        .build();
    let client = ElectrumClient::from_config(&wallet_settings.electrum().to_string(), config)?;

    sender
        .send(WatchMsg::Connected)
        .expect("electrum watcher channel is broken");

    let last_block = client.block_headers_subscribe()?;
    sender
        .send(WatchMsg::LastBlock(last_block))
        .expect("electrum watcher channel is broken");

    let fee = client.batch_estimate_fee([1, 2, 3])?;
    sender
        .send(WatchMsg::FeeEstimate(fee[0], fee[1], fee[2]))
        .expect("electrum watcher channel is broken");

    let network = bitcoin::Network::from(wallet_settings.network());

    let mut txids = bset![];
    let mut upto_index = map! { true => UnhardenedIndex::zero(), false => UnhardenedIndex::zero() };
    for change in [true, false] {
        let mut offset = 0u16;
        let mut upto = UnhardenedIndex::zero();
        *upto_index.entry(change).or_default() = loop {
            let spk = wallet_settings
                .script_pubkeys(change, offset..=(offset + 19))
                .map_err(|err| electrum_client::Error::Message(err.to_string()))?;
            let history_batch: Vec<_> = client
                .batch_script_get_history(spk.values().map(PubkeyScript::as_inner))?
                .into_iter()
                .zip(&spk)
                .flat_map(|(history, (index, script))| {
                    history.into_iter().map(move |res| HistoryTxid {
                        txid: res.tx_hash,
                        height: res.height,
                        address: AddressCompat::from_script(&script.clone().into(), network)
                            .expect("broken descriptor"),
                        index: *index,
                        change,
                    })
                })
                .collect();
            if history_batch.is_empty() {
                break upto;
            } else {
                upto = history_batch
                    .iter()
                    .map(|item| item.index)
                    .max()
                    .unwrap_or_default();
            }
            txids.extend(history_batch.iter().map(|item| item.txid));
            sender
                .send(WatchMsg::HistoryBatch(history_batch, offset))
                .expect("electrum watcher channel is broken");

            let utxos: Vec<_> = client
                .batch_script_list_unspent(spk.values().map(PubkeyScript::as_inner))?
                .into_iter()
                .zip(spk)
                .flat_map(|(utxo, (index, script))| {
                    utxo.into_iter().map(move |res| UtxoTxid {
                        txid: res.tx_hash,
                        height: res.height as u32,
                        pos: res.tx_pos as u32,
                        value: res.value,
                        address: AddressCompat::from_script(&script.clone().into(), network)
                            .expect("broken descriptor"),
                        index,
                        change,
                    })
                })
                .collect();
            txids.extend(utxos.iter().map(|item| item.txid));
            sender
                .send(WatchMsg::UtxoBatch(utxos, offset))
                .expect("electrum watcher channel is broken");

            offset += 20;
        };
    }
    let txids = txids.into_iter().collect::<Vec<_>>();
    for (no, chunk) in txids.chunks(20).enumerate() {
        let txmap = chunk
            .iter()
            .copied()
            .zip(client.batch_transaction_get(chunk)?)
            .collect::<BTreeMap<_, _>>();
        sender
            .send(WatchMsg::TxBatch(
                txmap,
                (no + 1) as f32 / txids.len() as f32 / 20.0,
            ))
            .expect("electrum watcher channel is broken");
    }

    // TODO: Subscribe to invoices

    sender
        .send(WatchMsg::Complete)
        .expect("electrum watcher channel is broken");

    loop {
        thread::sleep(Duration::from_secs(60));

        match client.block_headers_pop() {
            Ok(Some(last_block)) => sender.send(WatchMsg::LastBlockUpdate(last_block)),
            Err(err) => sender.send(WatchMsg::Error(err)),
            _ => Ok(()),
        }
        .expect("electrum watcher channel is broken");
    }
}