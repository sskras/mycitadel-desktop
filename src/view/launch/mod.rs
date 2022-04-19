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

use crate::model::{PublicNetwork, Requirement, WalletTemplate};
use gladis::Gladis;
use gtk::prelude::*;
use gtk::{Adjustment, ApplicationWindow, Button, ListBox, RecentChooserWidget, Switch};
use relm::{Relm, Update, Widget};

pub struct ViewModel {}

#[derive(Msg)]
pub enum Msg {
    Quit,
    TemplateSelected,
    ImportSelected,
    OpenSelected,
    RecentSelected,
}

#[derive(Clone, Gladis)]
pub struct Widgets {
    window: ApplicationWindow,
    action_btn: Button,
    hwcount_adj: Adjustment,
    taproot_swch: Switch,
    testnet_swch: Switch,
    rgb_swch: Switch,
    create_box: ListBox,
    import_box: ListBox,
    open_box: ListBox,
    recent: RecentChooserWidget,
}

pub struct Win {
    model: ViewModel,
    widgets: Widgets,
}

impl Win {
    fn is_taproot(&self) -> bool {
        self.widgets.taproot_swch.is_active()
    }

    fn network(&self) -> PublicNetwork {
        match self.widgets.testnet_swch.is_active() {
            true => PublicNetwork::Mainnet,
            false => PublicNetwork::Testnet,
        }
    }

    fn hide(&self) {
        self.widgets.window.hide()
    }

    fn open_template(&self) {
        let index = if let Some(row) = self.widgets.create_box.selected_row() {
            row.index()
        } else {
            return;
        };
        let taproot = self.is_taproot();
        let network = self.network();
        let template = match index {
            0 => Some(WalletTemplate::singlesig(taproot, network, false)),
            1 => Some(WalletTemplate::singlesig(taproot, network, true)),
            2 => Some(WalletTemplate::hodling(
                network,
                4,
                Requirement::Allow,
                Requirement::Allow,
            )),
            3 => {
                let count = self.widgets.hwcount_adj.value() as u16;
                Some(WalletTemplate::multisig(
                    network,
                    Some(count),
                    Requirement::Require,
                    Requirement::Deny,
                ))
            }
            4 => Some(WalletTemplate::multisig(
                network,
                None,
                Requirement::Allow,
                Requirement::Require,
            )),
            5 => Some(WalletTemplate::multisig(
                network,
                None,
                Requirement::Allow,
                Requirement::Allow,
            )),
            6 => None,
            _ => unreachable!("unknown template"),
        };

        // self.hide();
    }

    fn import_wallet(&self) {}

    fn open_file(&self) {}

    fn open_recent(&self) {}
}

impl Update for Win {
    // Specify the model used for this widget.
    type Model = ViewModel;
    // Specify the model parameter used to init the model.
    type ModelParam = ();
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(_relm: &Relm<Self>, _model: Self::ModelParam) -> Self::Model {
        ViewModel {}
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Quit => gtk::main_quit(),
            Msg::TemplateSelected => self.open_template(),
            Msg::ImportSelected => self.import_wallet(),
            Msg::OpenSelected => self.open_file(),
            Msg::RecentSelected => self.open_recent(),
        }
    }
}

impl Widget for Win {
    // Specify the type of the root widget.
    type Root = ApplicationWindow;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.window.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("launch.glade");
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        connect!(relm, widgets.action_btn, connect_clicked(_), Msg::Quit);

        connect!(
            relm,
            widgets.create_box,
            connect_row_selected(_, _),
            Msg::TemplateSelected
        );
        connect!(
            relm,
            widgets.import_box,
            connect_row_selected(_, _),
            Msg::ImportSelected
        );
        connect!(
            relm,
            widgets.open_box,
            connect_row_selected(_, _),
            Msg::OpenSelected
        );
        connect!(
            relm,
            widgets.recent,
            connect_selection_changed(_),
            Msg::RecentSelected
        );

        widgets.window.show();

        Win { widgets, model }
    }
}