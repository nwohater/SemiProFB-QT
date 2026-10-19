import QtQuick
import QtQuick.Layouts

// Title: stadium art, Continue (when there is a save) and New game.
Item {
    id: root
    signal newGame()
    signal continued()
    signal openSettings()

    Image {
        anchors.fill: parent
        source: Store.game.assetsUrl + "artwork/stadium-dusk-hero.png"
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        opacity: 0.55
    }
    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: "#cc16161e" }
            GradientStop { position: 0.6; color: "#9916161e" }
            GradientStop { position: 1.0; color: "#f016161e" }
        }
    }
    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 48, 460)
        spacing: 14
        Image {
            Layout.alignment: Qt.AlignHCenter
            source: Store.game.assetsUrl + "artwork/semi-pro-football-splash-logo.png"
            sourceSize.width: 640
            Layout.preferredWidth: Math.min(parent.width, 420)
            Layout.preferredHeight: Math.max(60, Math.min(420, root.height - 360))
            fillMode: Image.PreserveAspectFit
            asynchronous: true
        }
        Text {
            Layout.alignment: Qt.AlignHCenter
            text: "A small-town football management sim"
            color: Theme.inkMuted
            font.pixelSize: 15
        }
        Item { Layout.preferredHeight: 10 }
        PrimaryButton {
            Layout.fillWidth: true
            visible: Store.game.hasSave()
            text: "Continue"
            onClicked: { if (Store.game.load()) root.continued() }
        }
        PrimaryButton {
            Layout.fillWidth: true
            text: "New game"
            ghost: Store.game.hasSave()
            onClicked: root.newGame()
        }
        PrimaryButton {
            Layout.fillWidth: true
            text: "Settings"
            ghost: true
            onClicked: root.openSettings()
        }
        Text {
            Layout.alignment: Qt.AlignHCenter
            visible: Store.game.notice !== ""
            text: Store.game.notice
            color: Theme.badFg
            font.pixelSize: 13
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
        }
    }
}
