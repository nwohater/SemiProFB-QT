import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// League leaders: pick a stat group, sort by any column, switch regular season / playoffs and season / career.
ColumnLayout {
    id: root
    property string cat: "Passing"
    property bool playoffs: false
    property bool career: false
    property bool activeOnly: false
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.leadersJson(cat, playoffs, career, activeOnly)); }
    spacing: 10

    Flow {
        Layout.fillWidth: true; spacing: 6
        Repeater { model: root.vm.names || []; Chip { text: modelData; selected: root.cat === modelData; onClicked: root.cat = modelData } }
    }
    RowLayout {
        spacing: 14
        RowLayout {
            spacing: 6
            Chip { text: "Season"; selected: !root.career; onClicked: root.career = false }
            Chip { text: "Career"; selected: root.career; onClicked: root.career = true }
        }
        RowLayout {
            spacing: 6
            Chip { text: "Regular"; selected: !root.playoffs; onClicked: root.playoffs = false }
            Chip { text: "Playoffs"; selected: root.playoffs; onClicked: root.playoffs = true }
        }
        RowLayout {
            visible: root.career
            spacing: 8
            Check { checked: root.activeOnly; onToggled: root.activeOnly = checked }
            Text { text: "Active players only"; color: Theme.inkMuted; font.pixelSize: 13 }
        }
    }
    Card {
        title: root.cat + (root.career ? " career leaders" : " leaders")
        StatGrid {
            Layout.fillWidth: true
            table: root.vm
            empty: root.career ? "No career numbers yet." : (root.playoffs ? "No playoff numbers yet." : "No numbers yet. Play a game and check back.")
        }
    }
    Text {
        text: "Tap a column header to sort. Tap a player to open him. " + (root.career ? "Career totals through season " : "Season ") + (root.vm.season || "") + "."
        color: Theme.inkFaint; font.pixelSize: 12
    }
}
