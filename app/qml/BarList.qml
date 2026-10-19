import QtQuick
import QtQuick.Layouts

// Labelled horizontal bars for a revenue / expense breakdown. `items` = [{ name, amount, text }].
ColumnLayout {
    id: root
    property var items: []
    property color barColor: Theme.primary
    readonly property real total: { var t = 0; for (var i = 0; i < items.length; i++) t += Math.abs(items[i].amount); return t; }
    spacing: 6
    Repeater {
        model: root.items
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            RowLayout {
                Layout.fillWidth: true
                Text { text: modelData.name; color: Theme.inkMuted; font.pixelSize: 13; Layout.fillWidth: true }
                Text { text: modelData.text; color: Theme.ink; font.pixelSize: 13; font.bold: true }
            }
            Rectangle {
                Layout.fillWidth: true; height: 6; radius: 3; color: Theme.surfaceAlt
                Rectangle { width: root.total > 0 ? parent.width * Math.abs(modelData.amount) / root.total : 0; height: parent.height; radius: 3; color: root.barColor }
            }
        }
    }
}
