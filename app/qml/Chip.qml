import QtQuick
import QtQuick.Controls

// A selectable filter chip: tinted fill plus a primary border when selected.
AbstractButton {
    id: chip
    property bool selected: false
    checkable: false
    implicitHeight: 32
    leftPadding: 12
    rightPadding: 12
    font.pixelSize: 13
    font.bold: true
    contentItem: Text { text: chip.text; font: chip.font; color: chip.selected ? Theme.primary : Theme.inkMuted; verticalAlignment: Text.AlignVCenter; horizontalAlignment: Text.AlignHCenter }
    background: Rectangle {
        radius: Theme.radiusPill
        color: chip.selected ? Theme.primaryTint : (chip.hovered ? Theme.surfaceHi : Theme.surface)
        border.color: chip.selected ? Theme.primary : Theme.outline
        border.width: chip.selected ? 2 : 1
    }
    HoverHandler { cursorShape: Qt.PointingHandCursor }
}
