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

use std::str::FromStr;

use bitcoin::util::bip32::ExtendedPubKey;
use gladis::Gladis;
use gtk::MessageDialog;
use relm::{Relm, Update, Widget};
use wallet::slip132::FromSlip132;

use super::{Msg, ViewModel, Widgets};

pub struct Component {
    model: ViewModel,
    widgets: Widgets,
}

impl Update for Component {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = ();
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, _model: Self::ModelParam) -> Self::Model {
        ViewModel::default()
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Open(testnet, format) => {
                self.model.testnet = testnet;
                self.model.slip_format = format;
                self.widgets.open();
            }
            Msg::Edit => {
                let xpub = self.widgets.xpub();
                // TODO: Recognize Slip132 type and match with the wallet type
                match ExtendedPubKey::from_str(&xpub)
                    .or_else(|_| ExtendedPubKey::from_slip132_str(&xpub))
                {
                    Ok(_) => {
                        self.widgets.hide_message();
                        self.model.xpub = xpub
                    }
                    Err(err) => self.widgets.show_error(&err.to_string()),
                }
            }
            Msg::Error(msg) => self.widgets.show_error(&msg),
            Msg::Warning(msg) => self.widgets.show_warning(&msg),
            Msg::Info(msg) => self.widgets.show_info(&msg),
            Msg::Close => {}
            Msg::Ok => {}
        }
    }
}

impl Widget for Component {
    // Specify the type of the root widget.
    type Root = MessageDialog;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.to_root()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("xpub_dlg.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        widgets.connect(relm);

        Component { model, widgets }
    }
}