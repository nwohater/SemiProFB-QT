import QtQuick

// A small rounded label. Status pills always carry their word (Good / Okay / Risky), never color alone.
Rectangle {
    id: pill
    property alias text: label.text
    property color fg: Theme.inkMuted
    property color bg: Theme.surfaceAlt
    property bool strong: false
    color: bg
    radius: Theme.radiusPill
    implicitHeight: 24
    implicitWidth: label.implicitWidth + 18
    Text {
        id: label
        anchors.centerIn: parent
        color: pill.fg
        font.pixelSize: 12
        font.bold: pill.strong
    }
}
