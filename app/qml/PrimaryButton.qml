import QtQuick
import QtQuick.Controls

Button {
    id: btn
    property bool danger: false
    property bool ghost: false
    implicitHeight: 44
    leftPadding: 18
    rightPadding: 18
    font.pixelSize: 14
    font.bold: true
    contentItem: Text {
        text: btn.text
        font: btn.font
        color: !btn.enabled ? Theme.inkFaint : (btn.ghost ? Theme.ink : "#16161e")
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }
    background: Rectangle {
        radius: Theme.radiusButton
        color: !btn.enabled ? Theme.surfaceAlt : (btn.ghost ? (btn.hovered ? Theme.surfaceHi : "transparent") : (btn.danger ? Theme.badFg : (btn.down ? Theme.primaryDark : (btn.hovered ? "#8fb2fa" : Theme.primary))))
        border.color: btn.ghost ? Theme.outline : "transparent"
        border.width: btn.ghost ? 1 : 0
    }
    HoverHandler { cursorShape: Qt.PointingHandCursor }
}
