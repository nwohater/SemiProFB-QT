import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: home
    signal resultsReady(var r)
    signal go(int page)

    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.homeJson()); }
    readonly property var off: { Store.game.rev; return home.vm.phase === "Offseason" ? JSON.parse(Store.game.offseasonJson()) : ({}); }
    readonly property string phase: vm.phase || ""
    readonly property bool playing: phase === "RegularSeason" || phase === "Semifinals" || phase === "Championship"
    readonly property bool twoCol: width >= 900

    function primaryText() {
        switch (phase) {
        case "RegularSeason": return "Advance week";
        case "Semifinals": return "Play semifinals";
        case "Championship": return "Play championship";
        case "Complete": return "Begin offseason";
        case "Offseason": return "Open the offseason";
        }
        return "";
    }
    function primaryAction() {
        if (playing) {
            var r = JSON.parse(Store.game.advance());
            if (r.games) home.resultsReady(r);
        } else if (phase === "Complete") { Store.game.beginOffseason(); Store.openStory("retire"); }
        else if (phase === "Offseason") home.go(1);
    }

    ScrollView {
        id: sv
        anchors.fill: parent
        anchors.bottomMargin: 68
        contentWidth: availableWidth
        clip: true
        GridLayout {
            width: sv.availableWidth - 2 * Theme.gutter
            x: Theme.gutter
            y: Theme.gutter
            columns: home.twoCol ? 2 : 1
            columnSpacing: 12
            rowSpacing: 12

            // ---------------- left column
            ColumnLayout {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignTop
                Layout.preferredWidth: 3
                spacing: 12

                // champion banner
                Card {
                    visible: home.phase === "Complete" && !!home.vm.champion
                    fill: Theme.primaryTint
                    RowLayout {
                        spacing: 12
                        Helmet { slug: home.vm.champion ? home.vm.champion.helmet : ""; size: 56 }
                        ColumnLayout {
                            Layout.fillWidth: true
                            Text { text: home.vm.iWon ? "You are the champions!" : "Season complete"; color: Theme.gold; font.pixelSize: 20; font.bold: true }
                            Text { text: home.vm.champion ? ((home.vm.iWon ? "" : home.vm.champion.name + " won the title. ") + "Time to look ahead to the offseason.") : ""; color: Theme.inkMuted; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                        }
                    }
                }

                // offseason summary
                Card {
                    visible: home.phase === "Offseason"
                    title: "Offseason " + (home.off.season || "")
                    Text { text: "Champion: " + (home.off.champion || "–"); color: Theme.ink; font.pixelSize: 15; font.bold: true }
                    Text { text: (home.off.retired ? home.off.retired.length : 0) + " players retired league-wide  ·  " + (home.off.freeAgents || 0) + " free agents  ·  " + (home.off.expiring || 0) + " of your contracts expiring"; color: Theme.inkMuted; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    Text { visible: !!home.off.draft; text: home.off.draft ? ("Draft: " + home.off.draft.made + " of " + home.off.draft.total + " picks made  ·  you hold " + home.off.draft.myPicks + " picks") : ""; color: Theme.inkMuted; font.pixelSize: 13 }
                    Text { text: "Player development"; color: Theme.inkSubtle; font.pixelSize: 12; font.bold: true; topPadding: 6 }
                    Flow {
                        Layout.fillWidth: true
                        spacing: 6
                        Repeater {
                            model: (home.off.changes || []).filter(function(c) { return c.delta !== 0; }).sort(function(a, b) { return b.delta - a.delta; }).slice(0, 14)
                            Pill { text: modelData.name + "  " + (modelData.delta > 0 ? "+" : "") + modelData.delta; fg: modelData.delta > 0 ? Theme.goodFg : Theme.badFg; bg: modelData.delta > 0 ? Theme.goodBg : Theme.badBg }
                        }
                    }
                }

                // next game hero
                Card {
                    visible: !!home.vm.next
                    title: home.vm.next ? (home.vm.next.playoff ? "Playoff game" : "Week " + home.vm.next.week) + "  ·  " + (home.vm.next.home ? "Home" : "Away") : ""
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 14
                        Helmet { slug: home.vm.next ? home.vm.next.opponent.helmet : ""; size: 64 }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 0
                            Text { text: home.vm.next ? (home.vm.next.home ? "VS" : "AT") : ""; color: Theme.inkFaint; font.pixelSize: 12; font.bold: true; font.letterSpacing: 2 }
                            Text { text: home.vm.next ? home.vm.next.opponent.name : ""; color: Theme.ink; font.pixelSize: 24; font.bold: true; elide: Text.ElideRight; Layout.fillWidth: true }
                            Text { text: home.vm.next ? home.vm.next.opponent.record : ""; color: Theme.inkSubtle; font.pixelSize: 13 }
                        }
                    }
                    Rectangle {
                        visible: !!(home.vm.next && home.vm.next.rivalry)
                        Layout.fillWidth: true; implicitHeight: 30; radius: Theme.radiusChip; color: Theme.badBg
                        Text { anchors.centerIn: parent; text: "Rivalry game"; color: Theme.badFg; font.bold: true; font.pixelSize: 13 }
                    }
                    Text { visible: !!(home.vm.next && home.vm.next.quirk); text: home.vm.next ? home.vm.next.quirk : ""; color: Theme.inkSubtle; font.pixelSize: 12; font.italic: true; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    Text { text: home.vm.next ? home.vm.next.summary : ""; color: Theme.inkMuted; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    Flow {
                        Layout.fillWidth: true
                        spacing: 6
                        Repeater { model: home.vm.next ? home.vm.next.chips : []; Pill { text: modelData } }
                    }
                    Repeater {
                        model: home.vm.next ? home.vm.next.matchups : []
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            Pill { text: modelData.fit; fg: Theme.fitColor(modelData.fit); bg: Theme.fitBg(modelData.fit); strong: true; Layout.preferredWidth: 64 }
                            Text { text: modelData.label; color: Theme.ink; font.pixelSize: 13; font.bold: true; Layout.preferredWidth: 130; elide: Text.ElideRight }
                            Text { text: modelData.ours + "  vs  " + modelData.theirs; color: Theme.inkSubtle; font.pixelSize: 12; Layout.fillWidth: true; elide: Text.ElideRight }
                        }
                    }
                    Repeater {
                        model: home.vm.next ? home.vm.next.tips : []
                        Text { text: "•  " + modelData; color: Theme.inkMuted; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    }
                }

                // alerts
                Card {
                    id: alerts
                    readonly property var a: home.vm.alerts || ({ out: [], gaps: [], expiring: 0 })
                    visible: home.playing && (a.out.length > 0 || a.gaps.length > 0 || a.expiring > 0)
                    title: "Needs attention"
                    fill: Theme.warnBg
                    Repeater { model: alerts.a.out; Text { text: modelData.name + " (" + modelData.pos + ") is out " + modelData.games + " more game(s) but is on your active list."; color: Theme.warnFg; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true } }
                    Repeater { model: alerts.a.gaps; Text { text: "Roster gap — " + modelData; color: Theme.warnFg; font.pixelSize: 13 } }
                    Text { visible: alerts.a.expiring > 0; text: alerts.a.expiring + " contract(s) expire after this season."; color: Theme.warnFg; font.pixelSize: 13 }
                    PrimaryButton { text: "Auto-set active list"; ghost: true; implicitHeight: 34; onClicked: Store.game.autoSetActive() }
                }

                // last result
                Card {
                    visible: !!home.vm.last
                    title: home.vm.last ? "Last game  ·  Week " + home.vm.last.week : ""
                    TapHandler { enabled: !!home.vm.last && home.vm.last.box !== null; onTapped: Store.openBox(home.vm.last.box) }
                    HoverHandler { cursorShape: !!home.vm.last && home.vm.last.box !== null ? Qt.PointingHandCursor : Qt.ArrowCursor }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 12
                        Pill { text: home.vm.last ? home.vm.last.result : ""; strong: true
                            fg: home.vm.last && home.vm.last.result === "W" ? Theme.goodFg : (home.vm.last && home.vm.last.result === "L" ? Theme.badFg : Theme.warnFg)
                            bg: home.vm.last && home.vm.last.result === "W" ? Theme.goodBg : (home.vm.last && home.vm.last.result === "L" ? Theme.badBg : Theme.warnBg) }
                        Text { text: home.vm.last ? (home.vm.last.my + " – " + home.vm.last.their + (home.vm.last.ot ? " (OT)" : "")) : ""; color: Theme.ink; font.pixelSize: 22; font.bold: true }
                        Text { text: home.vm.last ? ((home.vm.last.home ? "vs " : "at ") + home.vm.last.opponent.name) : ""; color: Theme.inkMuted; font.pixelSize: 14; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { visible: !!(home.vm.last && home.vm.last.net !== null && home.vm.last.net !== undefined); text: home.vm.last && home.vm.last.net !== null ? ("Net " + Theme.money(home.vm.last.net)) : ""; color: home.vm.last && home.vm.last.net < 0 ? Theme.badFg : Theme.goodFg; font.pixelSize: 13 }
                    }
                    Text { visible: !!home.vm.last && home.vm.last.box !== null; text: "Tap for the box score ›"; color: Theme.inkFaint; font.pixelSize: 12 }
                }
            }

            // ---------------- right column
            ColumnLayout {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignTop
                Layout.preferredWidth: 2
                spacing: 12

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10
                    StatTile { label: "Cash"; value: home.vm.money ? home.vm.money.cashText : ""; accent: home.vm.money && home.vm.money.cash < 0 ? Theme.badFg : Theme.goodFg }
                    StatTile { label: "Last net"; value: home.vm.money && home.vm.money.lastNet !== null && home.vm.money.lastNet !== undefined ? Theme.money(home.vm.money.lastNet) : "–"; accent: home.vm.money && home.vm.money.lastNet < 0 ? Theme.badFg : Theme.ink }
                    StatTile { label: "Runway"; value: home.vm.money ? home.vm.money.runway : "" }
                }

                Card {
                    title: (home.vm.divisionName || "Division") + " standings"
                    Repeater {
                        model: home.vm.division || []
                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: 40
                            radius: Theme.radiusChip
                            color: modelData.isMe ? Theme.primaryTint : "transparent"
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 8; anchors.rightMargin: 8
                                spacing: 8
                                Text { text: modelData.rank; color: Theme.inkFaint; font.pixelSize: 13; Layout.preferredWidth: 14 }
                                Helmet { slug: modelData.team.helmet; size: 26 }
                                Text { text: modelData.team.name; color: modelData.isMe ? Theme.primary : Theme.ink; font.bold: modelData.isMe; font.pixelSize: 13; Layout.fillWidth: true; elide: Text.ElideRight }
                                Text { text: modelData.record; color: Theme.inkMuted; font.pixelSize: 13; font.bold: true }
                                Text { text: (modelData.diff > 0 ? "+" : "") + modelData.diff; color: modelData.diff >= 0 ? Theme.goodFg : Theme.badFg; font.pixelSize: 12; Layout.preferredWidth: 36; horizontalAlignment: Text.AlignRight }
                            }
                        }
                    }
                }
                Text {
                    visible: home.playing && home.vm.rank !== undefined
                    text: "You're " + ["", "1st", "2nd", "3rd", "4th", "5th", "6th"][Math.min(home.vm.rank || 0, 6)] + " in your division"
                    color: Theme.inkSubtle; font.pixelSize: 12
                }
            }
        }
    }

    // ---- bottom action bar
    Rectangle {
        anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
        height: 68
        color: Theme.bg
        Rectangle { width: parent.width; height: 1; color: Theme.divider }
        PrimaryButton {
            anchors.centerIn: parent
            width: Math.min(parent.width - 2 * Theme.gutter, 420)
            implicitHeight: 46
            text: home.primaryText()
            visible: text !== "" && !(home.vm.header && home.vm.header.gameOver)
            onClicked: home.primaryAction()
        }
        Text {
            anchors.centerIn: parent
            visible: !!(home.vm.header && home.vm.header.gameOver)
            text: "Game over — the franchise has run out of money."
            color: Theme.badFg; font.pixelSize: 15; font.bold: true
        }
    }
}
