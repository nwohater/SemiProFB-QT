import QtQuick
import QtQuick.Controls

// Themed checkbox: outlined square, primary fill and a check mark when on.
CheckBox {
    id: control
    padding: 4
    implicitWidth: 28
    implicitHeight: 28
    indicator: Rectangle {
        implicitWidth: 20; implicitHeight: 20
        x: control.leftPadding; y: control.topPadding + (control.availableHeight - height) / 2
        radius: 5
        color: control.checked ? Theme.primary : "transparent"
        border.color: !control.enabled ? Theme.divider : (control.checked ? Theme.primary : Theme.outline)
        border.width: 1.5
        Text { anchors.centerIn: parent; visible: control.checked; text: "✓"; color: Theme.bg; font.pixelSize: 14; font.bold: true }
    }
    contentItem: Item {}
    HoverHandler { cursorShape: Qt.PointingHandCursor }
}
