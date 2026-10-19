import QtQuick
import QtQuick.Layouts

// Past champions, newest first.
ColumnLayout {
    id: root
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.leagueTabJson("history")); }
    function ordinal(n) {
        if (n === 1) return "";
        var m = n % 100;
        var s = (m >= 11 && m <= 13) ? "th" : ({ 1: "st", 2: "nd", 3: "rd" }[n % 10] || "th");
        return n + s + " title";
    }
    spacing: 8

    Card {
        visible: (root.vm.rows || []).length === 0
        Text { text: "No completed seasons yet."; color: Theme.inkMuted; font.pixelSize: 13 }
    }
    RowLayout {
        visible: (root.vm.rows || []).length > 0
        Layout.fillWidth: true
        Text { text: "Past champions"; color: Theme.ink; font.pixelSize: 16; font.bold: true; Layout.fillWidth: true }
        Text {
            visible: !!root.vm.dynasty && root.vm.dynasty.titles > 1
            text: root.vm.dynasty ? "Most titles: " + root.vm.dynasty.name + " (" + root.vm.dynasty.titles + ")" : ""
            color: Theme.inkSubtle; font.pixelSize: 12
        }
    }
    Repeater {
        model: root.vm.rows || []
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 72
            radius: Theme.radiusCard
            color: modelData.isYou ? Theme.primaryTint : Theme.surface
            border.color: modelData.isYou ? Theme.primary : Theme.divider
            border.width: modelData.isYou ? 2 : 1
            clip: true
            Rectangle { width: 5; height: parent.height; color: Theme.gold }
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 17; anchors.rightMargin: 14
                spacing: 12
                Helmet { slug: modelData.helmet; size: 52 }
                ColumnLayout {
                    Layout.fillWidth: true; spacing: 2
                    Text { text: "SEASON " + modelData.season; color: Theme.warnFg; font.pixelSize: 13; font.bold: true; font.letterSpacing: 1.2 }
                    Text { text: modelData.name; color: Theme.ink; font.pixelSize: 16; font.bold: true; elide: Text.ElideRight; Layout.fillWidth: true }
                    RowLayout {
                        spacing: 8
                        Text { text: modelData.record; color: Theme.inkSubtle; font.pixelSize: 13 }
                        Text { visible: root.ordinal(modelData.title) !== ""; text: root.ordinal(modelData.title); color: Theme.warnFg; font.pixelSize: 12; font.bold: true }
                        Text { visible: modelData.isYou; text: "Your team"; color: Theme.primary; font.pixelSize: 12; font.bold: true }
                    }
                }
                Text { text: "🏆"; font.pixelSize: 26 }
            }
        }
    }
}
