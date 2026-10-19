import QtQuick
import QtQuick.Controls

// Themed slider: thin track, primary fill, round handle.
Slider {
    id: control
    implicitHeight: 28
    background: Rectangle {
        x: control.leftPadding
        y: control.topPadding + control.availableHeight / 2 - height / 2
        width: control.availableWidth; height: 5; radius: 3
        color: Theme.surfaceHi
        Rectangle { width: control.visualPosition * parent.width; height: parent.height; radius: 3; color: Theme.primary }
    }
    handle: Rectangle {
        x: control.leftPadding + control.visualPosition * (control.availableWidth - width)
        y: control.topPadding + control.availableHeight / 2 - height / 2
        width: 18; height: 18; radius: 9
        color: control.pressed ? Theme.primaryDark : Theme.primary
        border.color: Theme.bg; border.width: 2
    }
    HoverHandler { cursorShape: Qt.PointingHandCursor }
}
