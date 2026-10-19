import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// One team's numbers: totals with league rank, then every stat group.
ColumnLayout {
    id: root
    property int team: -1
    property bool playoffs: false
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.teamStatsJson(team, playoffs)); }
    spacing: 10

    RowLayout {
        Layout.fillWidth: true; spacing: 12
        PrimaryButton { text: "‹"; ghost: true; implicitHeight: 34; leftPadding: 12; rightPadding: 12; onClicked: root.team = (root.vm.team.index + root.vm.count - 1) % root.vm.count }
        Helmet { slug: root.vm.team ? root.vm.team.helmet : ""; size: 48 }
        ColumnLayout {
            spacing: 0
            Text { text: root.vm.team ? root.vm.team.name : ""; color: Theme.ink; font.pixelSize: 20; font.bold: true }
            Text {
                text: root.vm.team ? root.vm.team.record + "  ·  #" + root.vm.rank + " of " + root.vm.count + "  ·  PF " + root.vm.pf + " · PA " + root.vm.pa : ""
                color: Theme.inkSubtle; font.pixelSize: 13
            }
        }
        PrimaryButton { text: "›"; ghost: true; implicitHeight: 34; leftPadding: 12; rightPadding: 12; onClicked: root.team = (root.vm.team.index + 1) % root.vm.count }
        ComboBox {
            id: pick
            Layout.preferredWidth: 220
            model: root.vm.teams || []
            textRole: "name"
            currentIndex: root.vm.team ? root.vm.team.index : 0
            onActivated: function(i) { root.team = i; }
        }
        Item { Layout.fillWidth: true }
        RowLayout {
            spacing: 6
            Chip { text: "Regular"; selected: !root.playoffs; onClicked: root.playoffs = false }
            Chip { text: "Playoffs"; selected: root.playoffs; onClicked: root.playoffs = true }
        }
    }
    Text { text: "Totals are rostered players only."; color: Theme.inkFaint; font.pixelSize: 12 }

    GridLayout {
        Layout.fillWidth: true
        columns: root.width >= 900 ? 3 : 1
        columnSpacing: 12; rowSpacing: 12
        Repeater {
            model: root.vm.totals || []
            Card {
                Layout.alignment: Qt.AlignTop
                Layout.preferredWidth: 1
                title: modelData.title
                Repeater {
                    model: modelData.rows
                    RowLayout {
                        Layout.fillWidth: true
                        Text { text: modelData.label; color: Theme.inkMuted; font.pixelSize: 13; Layout.fillWidth: true }
                        Text { text: modelData.text; color: Theme.ink; font.pixelSize: 13; font.bold: true }
                        Text {
                            text: "#" + modelData.rank; Layout.preferredWidth: 40; horizontalAlignment: Text.AlignRight
                            font.pixelSize: 12; font.bold: true
                            color: modelData.tone === "good" ? Theme.goodFg : (modelData.tone === "bad" ? Theme.badFg : Theme.inkFaint)
                        }
                    }
                }
            }
        }
    }
    Repeater {
        model: root.vm.tables || []
        Card {
            title: modelData.name
            StatGrid { Layout.fillWidth: true; table: modelData; pageSize: 60; showTeam: false; empty: "Nobody on this team has numbers here yet." }
        }
    }
}
