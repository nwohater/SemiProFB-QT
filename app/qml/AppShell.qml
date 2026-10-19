import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// The running game: persistent team header, navigation (side rail when wide, tabs when narrow), the screens.
Item {
    id: shell
    signal quit()

    property int page: 0
    readonly property bool wide: width >= 760
    readonly property var hdr: { Store.game.rev; return JSON.parse(Store.game.headerJson()); }
    readonly property string phase: hdr.phase || ""
    readonly property var allPages: [ { name: "Home", icon: "⌂", idx: 0 }, { name: "Offseason", icon: "❄", idx: 1 }, { name: "Inbox", icon: "✉", idx: 5 }, { name: "Roster", icon: "☰", idx: 2 }, { name: "Depth chart", icon: "≡", idx: 8 }, { name: "Game plan", icon: "✎", idx: 7 }, { name: "Free agents", icon: "＋", idx: 3 }, { name: "Trades", icon: "⇄", idx: 9 }, { name: "Finances", icon: "$", idx: 6 }, { name: "League", icon: "🏆", idx: 4 }, { name: "Gazette", icon: "📰", idx: 10 }, { name: "Legacy", icon: "★", idx: 13 }, { name: "Settings", icon: "⚙", idx: 11 }, { name: "Help", icon: "?", idx: 12 } ]
    readonly property var pages: allPages.filter(function(p) { return p.name !== "Offseason" || shell.phase === "Offseason"; })
    onPhaseChanged: { if (phase === "Offseason") page = 1; else if (page === 1) page = 0; }

    // dev hooks: --dev-page N, --dev-advance N
    Component.onCompleted: {
        var args = Qt.application.arguments;
        var i = args.indexOf("--dev-advance");
        if (i >= 0) for (var k = 0; k < parseInt(args[i + 1]); k++) Store.game.advance();
        i = args.indexOf("--dev-seasons");
        if (i >= 0) for (var sn = 0; sn < parseInt(args[i + 1]); sn++) {
            for (var g2 = 0; g2 < 20 && ["RegularSeason", "Semifinals", "Championship"].indexOf(JSON.parse(Store.game.headerJson()).phase) >= 0; g2++) Store.game.advance();
            Store.game.beginOffseason(); Store.game.simDraftAll(); Store.game.finishOffseason();
        }
        if (args.indexOf("--dev-offseason") >= 0) {
            for (var g = 0; g < 20 && ["RegularSeason", "Semifinals", "Championship"].indexOf(JSON.parse(Store.game.headerJson()).phase) >= 0; g++) Store.game.advance();
            Store.game.beginOffseason();
            if (args.indexOf("--dev-nextseason") >= 0) { Store.game.simDraftAll(); console.log("NEXT", Store.game.finishOffseason(), JSON.stringify(JSON.parse(Store.game.headerJson()).phase)); }
        }
        if (args.join(" ").indexOf("--dev-") < 0 && JSON.parse(Store.game.settingsJson()).tutorialSeen !== true) Qt.callLater(function() { tutorial.show(); });
        if (args.indexOf("--dev-tutorial") >= 0) Qt.callLater(function() { tutorial.show(); });
        i = args.indexOf("--dev-story");
        if (i >= 0) { var storyKind = args[i + 1]; Qt.callLater(function() { Store.openStory(storyKind); }); }
        i = args.indexOf("--dev-page");
        if (i >= 0) page = parseInt(args[i + 1]);
        i = args.indexOf("--dev-dialog");
        if (i >= 0) Qt.callLater(function() {
            if (args[i + 1] === "player") Store.openPlayer(JSON.parse(Store.game.rosterJson()).rows[0].id);
            else if (args[i + 1] === "other") Store.openPlayer(JSON.parse(Store.game.leadersJson("Passing", false, true, false)).rows[0].id);
            else if (args[i + 1] === "inbox") { var its = JSON.parse(Store.game.inboxJson()).items.filter(function(x) { return x.playerId !== null; }); if (its.length) Store.openPlayer(its[0].playerId); }
            else if (args[i + 1] === "box") {
                var gs = JSON.parse(Store.game.leagueTabJson("schedule")).games.filter(function(g) { return g.box !== null; });
                if (gs.length) Store.openBox(gs[0].box);
            } else Store.openNegotiation(JSON.parse(Store.game.rosterJson()).rows[0].id);
        });
    }

    function showResults(r) { results.show(r); }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // ---- persistent team header
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 64
            color: Theme.surface
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.divider }
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14; anchors.rightMargin: 14
                spacing: 12
                Helmet { slug: shell.hdr.team ? shell.hdr.team.helmet : ""; size: 44 }
                ColumnLayout {
                    spacing: 0
                    Layout.fillWidth: true
                    Text { text: shell.hdr.team ? shell.hdr.team.name : ""; color: Theme.ink; font.pixelSize: 17; font.bold: true; elide: Text.ElideRight; Layout.fillWidth: true }
                    Text { text: shell.hdr.team ? ("Season " + shell.hdr.season + "  ·  " + shell.hdr.weekText + "  ·  " + shell.hdr.team.record) : ""; color: Theme.inkSubtle; font.pixelSize: 12; elide: Text.ElideRight; Layout.fillWidth: true }
                }
                Pill { visible: shell.width >= 520; text: "Cash " + (shell.hdr.cashText || ""); fg: shell.hdr.cash < 0 ? Theme.badFg : Theme.goodFg; bg: shell.hdr.cash < 0 ? Theme.badBg : Theme.goodBg; strong: true }
                PrimaryButton { text: "Menu"; ghost: true; implicitHeight: 34; onClicked: menu.open()
                    Menu {
                        id: menu
                        y: parent.height
                        MenuItem { text: "Save and quit to title"; onTriggered: { Store.game.quitToTitle(); shell.quit(); } }
                        MenuItem { text: "Auto-set my active list"; onTriggered: Store.game.autoSetActive() }
                    }
                }
            }
        }

        // ---- body: rail + content (wide) or tabs + content (narrow)
        GridLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            columns: shell.wide ? 2 : 1
            rowSpacing: 0
            columnSpacing: 0

            Rectangle {
                Layout.fillHeight: shell.wide
                Layout.fillWidth: !shell.wide
                Layout.preferredWidth: shell.wide ? 176 : -1
                Layout.preferredHeight: shell.wide ? -1 : 44
                color: Theme.bg
                Rectangle { visible: shell.wide; anchors.right: parent.right; width: 1; height: parent.height; color: Theme.divider }
                Rectangle { visible: !shell.wide; anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.divider }
                GridLayout {
                    anchors.fill: parent
                    anchors.margins: 8
                    columns: shell.wide ? 1 : shell.pages.length
                    rowSpacing: 4; columnSpacing: 4
                    Repeater {
                        model: shell.pages
                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: shell.wide ? 42 : 34
                            radius: Theme.radiusButton
                            color: shell.page === modelData.idx ? Theme.primaryTint : (hov.hovered ? Theme.surface : "transparent")
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 10; anchors.rightMargin: 10
                                spacing: 10
                                Text { text: modelData.icon; color: shell.page === modelData.idx ? Theme.primary : Theme.inkSubtle; font.pixelSize: 16; horizontalAlignment: Text.AlignHCenter; Layout.preferredWidth: 22 }
                                Text { text: modelData.name; color: shell.page === modelData.idx ? Theme.primary : Theme.inkMuted; font.pixelSize: 14; font.bold: shell.page === modelData.idx; Layout.fillWidth: true }
                                Rectangle { visible: modelData.name === "Inbox" && shell.hdr.unread > 0; width: Math.max(18, badgeText.implicitWidth + 10); height: 18; radius: 9; color: Theme.primary
                                    Text { id: badgeText; anchors.centerIn: parent; text: shell.hdr.unread > 99 ? "99+" : shell.hdr.unread; color: "#16161e"; font.pixelSize: 10; font.bold: true } }
                            }
                            HoverHandler { id: hov; cursorShape: Qt.PointingHandCursor }
                            TapHandler { onTapped: shell.page = modelData.idx }
                        }
                    }
                    Item { Layout.fillHeight: true; visible: shell.wide }
                }
            }

            // Content stops growing on very wide (tiled / ultrawide) windows and sits in the middle.
            StackLayout {
                Layout.fillWidth: true
                Layout.maximumWidth: 1560
                Layout.alignment: Qt.AlignHCenter
                Layout.fillHeight: true
                currentIndex: shell.page
                HomeScreen { onResultsReady: function(r) { shell.showResults(r) } onGo: function(i) { shell.page = i } }
                OffseasonScreen {}
                RosterScreen {}
                FreeAgentsScreen {}
                LeagueScreen {}
                InboxScreen {}
                FinancesScreen {}
                StrategyScreen {}
                DepthScreen {}
                TradesScreen {}
                NewsScreen {}
                SettingsScreen { onAbandoned: shell.quit(); onReplayTutorial: tutorial.show() }
                HelpScreen {}
                LegacyScreen {}
            }
        }
    }

    WeekResultsDialog {
        id: results; parent: Overlay.overlay
        onClosed: {
            if (res.header && res.header.gameOver) Store.openStory("gameover");
            else if (res.phaseAfter === "Complete" && JSON.parse(Store.game.storyJson("champion")).name) Store.openStory("champion");
        }
    }
    PlayerDialog { id: playerDlg; parent: Overlay.overlay }
    NegotiationDialog { id: negDlg; parent: Overlay.overlay }
    BoxScoreDialog { id: boxDlg; parent: Overlay.overlay }
    StoryOverlay { id: storyDlg }
    TutorialOverlay { id: tutorial; onClosed: Store.game.setSetting("tutorialSeen", "true") }
    Connections {
        target: Store
        function onPlayerRequested(id) { playerDlg.show(id); }
        function onNegotiateRequested(id) { negDlg.show(id); }
        function onBoxRequested(i) { boxDlg.show(i); }
        function onStoryRequested(k) { storyDlg.show(k); }
    }
}
