import QtQuick
import QtQuick.Layouts

Rectangle {
    id: tile
    property string label: ""
    property string value: ""
    property color accent: Theme.ink
    color: Theme.surfaceAlt
    radius: Theme.radiusChip + 2
    implicitHeight: 64
    Layout.fillWidth: true
    Layout.minimumWidth: 96
    Column {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 2
        Text { text: tile.label; color: Theme.inkSubtle; font.pixelSize: 12 }
        Text { text: tile.value; color: tile.accent; font.pixelSize: 22; font.bold: true; elide: Text.ElideRight; width: parent.width }
    }
}
