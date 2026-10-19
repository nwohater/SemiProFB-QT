import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// First-run walkthrough: a few full-window cards. Skippable; replayable from Settings.
Popup {
    id: dlg
    property int page: 0
    readonly property var pages: [
        { icon: "🏟", kicker: "WELCOME, OWNER", title: "You run a small-town football team", color: "#1c3a5e", points: [
            "A 12-team league in California, Texas, Florida or Pennsylvania, 14 games, then a 4-team playoff.",
            "Your goals: survive, then win a title. The town council is watching.",
            "You never call plays. You build the roster, set the plan, and read the box score." ] },
        { icon: "📅", kicker: "EVERY WEEK", title: "Home is your game-week hub", color: "#1e4a52", points: [
            "See your next opponent, how your units match up, and what needs attention.",
            "Fix the alerts (injuries, a missing punter, a short roster), then press Play.",
            "After the game, open the box score. It tells you why you won or lost." ] },
        { icon: "👥", kicker: "THE ROSTER", title: "46 dress, the rest get a retainer", color: "#2b3f6b", points: [
            "Only 46 players are active on game day. Active players get paid per game.",
            "Inactive players cost a small retainer. Auto-set picks your best 46 each week.",
            "Use the Depth chart to say who starts at each position. Click a player for his bio." ] },
        { icon: "🧭", kicker: "THE GAME PLAN", title: "Strategy and fit", color: "#2f4a3a", points: [
            "Pick a scheme and set the sliders: run or pass, how deep, how often to blitz.",
            "Fit chips say Good, Okay or Risky for your roster. Deep passing needs an arm and fast receivers.",
            "Check the opponent report on Home before you decide." ] },
        { icon: "💵", kicker: "THE MONEY", title: "Stay out of the red", color: "#4a3a1c", points: [
            "Tickets, concessions, merch, sponsors and league share pay the bills. Payroll is the big cost.",
            "Pull the levers: ticket price, promo nights, sponsors, stadium upgrades, loans.",
            "Three overdrawn game days in a row and the bank ends your franchise." ] },
        { icon: "🛠", kicker: "REAL LIFE", title: "Your players have day jobs", color: "#5a2f2a", points: [
            "They can get hurt at work or get stuck on a work trip and miss a game.",
            "Local players are cheaper. Fan favorites bring in crowds.",
            "The Inbox collects reports, sponsor news, bank letters, and the players' own excuses. The Gazette tells the story of the league." ] },
        { icon: "🔁", kicker: "EVERY YEAR", title: "The offseason", color: "#1c3a5e", points: [
            "Players improve or decline, and some retire. You see before and after, never hidden potential.",
            "Contracts expire. Re-sign people or let them walk. Then comes the draft, where you pick fresh talent at a fixed rookie pay. Free agents and tryouts are open any time.",
            "Stuck? Open Help in the side rail. Every rule is in there." ] }
    ]
    readonly property bool last: page === pages.length - 1
    function show() { page = 0; open(); }

    parent: Overlay.overlay
    x: 0; y: 0
    width: parent ? parent.width : 800
    height: parent ? parent.height : 600
    padding: 0
    modal: true
    closePolicy: Popup.CloseOnEscape
    Overlay.modal: Item {}
    background: Rectangle { color: dlg.pages[dlg.page].color; Behavior on color { ColorAnimation { duration: 200 } } }

    contentItem: ColumnLayout {
        spacing: 0
        RowLayout {
            Layout.fillWidth: true
            Item { Layout.fillWidth: true }
            PrimaryButton { visible: !dlg.last; text: "Skip"; ghost: true; implicitHeight: 38; Layout.margins: 12; onClicked: dlg.close() }
            Item { visible: dlg.last; Layout.preferredHeight: 38; Layout.margins: 12 }
        }
        Item {
            Layout.fillWidth: true; Layout.fillHeight: true
            ColumnLayout {
                anchors.centerIn: parent
                width: Math.min(parent.width - 56, 560)
                spacing: 14
                Rectangle {
                    width: 84; height: 84; radius: 42; color: "#29ffffff"
                    Text { anchors.centerIn: parent; text: dlg.pages[dlg.page].icon; font.pixelSize: 40 }
                }
                Item { Layout.preferredHeight: 6 }
                Text { text: dlg.pages[dlg.page].kicker; color: "#d5e0f5"; font.pixelSize: 13; font.bold: true; font.letterSpacing: 1.4 }
                Text { text: dlg.pages[dlg.page].title; color: "white"; font.pixelSize: 38; font.bold: true; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                Item { Layout.preferredHeight: 6 }
                Repeater {
                    model: dlg.pages[dlg.page].points
                    RowLayout {
                        Layout.fillWidth: true; spacing: 12
                        Rectangle { width: 7; height: 7; radius: 4; color: Theme.gold; Layout.alignment: Qt.AlignTop; Layout.topMargin: 9 }
                        Text { text: modelData; color: "white"; font.pixelSize: 17; wrapMode: Text.WordWrap; Layout.fillWidth: true; lineHeight: 1.2 }
                    }
                }
            }
        }
        RowLayout {
            Layout.fillWidth: true; Layout.margins: 20; spacing: 12
            Item {
                Layout.preferredWidth: 90
                PrimaryButton { visible: dlg.page > 0; text: "Back"; ghost: true; implicitHeight: 44; onClicked: dlg.page-- }
            }
            Item { Layout.fillWidth: true }
            Row {
                spacing: 6
                Repeater {
                    model: dlg.pages.length
                    Rectangle { width: index === dlg.page ? 20 : 8; height: 8; radius: 4; color: index === dlg.page ? "white" : "#59ffffff" }
                }
            }
            Item { Layout.fillWidth: true }
            PrimaryButton { Layout.preferredWidth: 130; implicitHeight: 44; text: dlg.last ? "Let's play" : "Next"; onClicked: { if (dlg.last) dlg.close(); else dlg.page++; } }
        }
    }
}
