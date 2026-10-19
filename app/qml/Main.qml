import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.semipro

ApplicationWindow {
    id: win
    visible: true
    width: 1180
    height: 760
    minimumWidth: 320
    minimumHeight: 360
    title: "Semi-Pro Football"
    color: Theme.bg

    // "title" | "new" | "game"
    property string screen: Store.game.hasGame ? "game" : "title"
    font.family: Theme.bodyFamilies[0]

    // dev hooks: --dev-new starts a game immediately
    Component.onCompleted: { if (Qt.application.arguments.join(" ").indexOf("--dev-") < 0) Music.start(); Qt.callLater(function() { if (Qt.application.arguments.indexOf("--dev-new") >= 0 && !Store.game.hasGame) newGameScreen.start(); else if (Qt.application.arguments.join(" ").indexOf("--dev-") >= 0 && !Store.game.hasGame && Store.game.load()) win.screen = "game"; }); }

    // Quitting to the title drops the game, which tears the shell down before it can say so: follow the game itself.
    Connections {
        target: Store.game
        function onHasGameChanged() { if (!Store.game.hasGame && win.screen === "game") win.screen = "title"; }
    }

    StackLayout {
        anchors.fill: parent
        currentIndex: win.screen === "title" ? 0 : (win.screen === "new" ? 1 : 2)
        TitleScreen {
            onNewGame: win.screen = "new"
            onContinued: win.screen = "game"
            onOpenSettings: titleSettings.open()
        }
        NewGameScreen {
            id: newGameScreen
            onBack: win.screen = "title"
            onStarted: win.screen = "game"
        }
        Loader {
            active: Store.game.hasGame
            sourceComponent: AppShell { onQuit: win.screen = "title" }
        }
    }

    // Settings before a franchise exists: sound and about.
    Popup {
        id: titleSettings
        parent: Overlay.overlay
        x: 0; y: 0; width: parent ? parent.width : 800; height: parent ? parent.height : 600
        padding: 0
        modal: true
        closePolicy: Popup.CloseOnEscape
        background: Rectangle { color: Theme.bg }
        Overlay.modal: Item {}
        contentItem: Item {
            SettingsScreen { anchors.fill: parent; anchors.topMargin: 52 }
            PrimaryButton { anchors.right: parent.right; anchors.top: parent.top; anchors.margins: 12; text: "Close"; ghost: true; implicitHeight: 34; onClicked: titleSettings.close() }
        }
    }
}
