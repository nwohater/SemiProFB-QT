import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    property int partner: -1
    property string pos: "All"
    property var getPlayers: []
    property var givePlayers: []
    property var getPicks: []
    property var givePicks: []
    property bool sendCash: true
    property int cash: 0
    property string notice: ""
    property var infoLine: ({})

    readonly property var desk: { Store.game.rev; return JSON.parse(Store.game.tradeDeskJson(root.partner)); }
    readonly property var offer: ({ partner: desk.partner, givePlayers: givePlayers, getPlayers: getPlayers, givePicks: givePicks, getPicks: getPicks, giveCash: sendCash ? cash : 0, getCash: sendCash ? 0 : cash })
    readonly property var verdict: { Store.game.rev; return JSON.parse(Store.game.tradeEvalJson(JSON.stringify(offer))); }
    readonly property var groupMap: ({ "RB": ["RB", "FB"], "OL": ["OT", "OG", "C"], "DL": ["DE", "DT"], "LB": ["MLB", "OLB"], "DB": ["CB", "S"] })
    function inGroup(p) { return pos === "All" || (groupMap[pos] || [pos]).indexOf(p.pos) >= 0; }
    function has(list, x) { return list.indexOf(x) >= 0; }
    function toggled(list, x, on) { var l = list.filter(function(v) { return v !== x; }); if (on) l.push(x); return l; }
    function reset() { getPlayers = []; givePlayers = []; getPicks = []; givePicks = []; cash = 0; sendCash = true; notice = ""; }
    function addAsk(a) {
        if (a.kind === "player") givePlayers = toggled(givePlayers, a.playerId, true);
        else if (a.kind === "pick") givePicks = toggled(givePicks, a.pick, true);
        else { if (!sendCash) { sendCash = true; cash = 0; } cash += a.cash; }
    }
    function propose() {
        var err = Store.game.proposeTrade(JSON.stringify(offer));
        notice = err === "" ? "Trade done with " + desk.partnerInfo.team.name + ". Check your roster." : err;
        if (err === "") reset();
    }
    Component.onCompleted: { var i = Qt.application.arguments.indexOf("--dev-trade"); if (i >= 0) { var d = JSON.parse(Store.game.tradeDeskJson(-1)); getPlayers = [d.getPlayers[0].id]; givePlayers = [d.givePlayers[0].id]; } }

    component AssetList: ColumnLayout {
        id: al
        property var players: []
        property var picks: []
        property var chosenPlayers: []
        property var chosenPicks: []
        signal playerToggled(int id, bool on)
        signal pickToggled(string key, bool on)
        spacing: 4
        Text { visible: al.picks.length > 0; text: "Draft picks"; color: Theme.inkSubtle; font.pixelSize: 12; font.bold: true }
        Flow {
            Layout.fillWidth: true
            spacing: 5
            Repeater {
                model: al.picks
                Chip {
                    text: modelData.label.replace("Season ", "S").replace(" round ", " R") + " · " + modelData.tier
                    selected: root.has(al.chosenPicks, modelData.key)
                    implicitHeight: 28; font.pixelSize: 12
                    onClicked: al.pickToggled(modelData.key, !selected)
                    ToolTip.visible: hovered; ToolTip.text: modelData.note
                }
            }
        }
        Text { visible: al.players.length > 0; text: "Players"; color: Theme.inkSubtle; font.pixelSize: 12; font.bold: true; topPadding: 4 }
        Repeater {
            model: al.players.filter(root.inGroup)
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 48
                radius: Theme.radiusChip
                color: root.has(al.chosenPlayers, modelData.id) ? Theme.primaryTint : (hv.hovered ? Theme.surfaceAlt : "transparent")
                opacity: modelData.lock ? 0.55 : 1
                HoverHandler { id: hv }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 4; anchors.rightMargin: 6; spacing: 6
                    Check { checked: root.has(al.chosenPlayers, modelData.id); enabled: !modelData.lock; onToggled: al.playerToggled(modelData.id, checked); padding: 4 }
                    ColumnLayout { Layout.fillWidth: true; spacing: 0
                        Text { text: modelData.pos + " " + modelData.name; color: Theme.ink; font.bold: true; font.pixelSize: 13; elide: Text.ElideRight; Layout.fillWidth: true }
                        Text { text: modelData.lock ? modelData.lock : "Age " + modelData.age + " · " + modelData.deal; color: modelData.lock ? Theme.warnFg : Theme.inkSubtle; font.pixelSize: 11; elide: Text.ElideRight; Layout.fillWidth: true }
                    }
                    Text { visible: modelData.out > 0; text: "OUT " + modelData.out; color: Theme.badFg; font.pixelSize: 11; font.bold: true }
                    Text { text: modelData.ovr; color: Theme.ovrColor(modelData.ovr); font.bold: true; font.pixelSize: 15; Layout.preferredWidth: 26 }
                    PrimaryButton { text: "i"; ghost: true; implicitHeight: 26; implicitWidth: 28; leftPadding: 4; rightPadding: 4; font.pixelSize: 12; onClicked: { root.infoLine = modelData.line; infoDlg.open(); } }
                }
            }
        }
    }

    ScrollView {
        id: sv
        anchors.fill: parent
        contentWidth: availableWidth
        clip: true
        ColumnLayout {
            width: sv.availableWidth - 2 * Theme.gutter
            x: Theme.gutter; y: Theme.gutter
            spacing: 12
            RowLayout {
                Layout.fillWidth: true
                ScreenTitle { text: "Trades"; Layout.fillWidth: true }
                Text { text: root.desk.windowText || ""; color: root.desk.closed ? Theme.warnFg : Theme.inkSubtle; font.pixelSize: 12; visible: root.width >= 560 }
            }

            // partner picker
            Card {
                title: "Trade partner"
                Flow { Layout.fillWidth: true; spacing: 6
                    Repeater { model: root.desk.teams || []; Chip { text: modelData.town; selected: modelData.index === root.desk.partner; onClicked: { root.partner = modelData.index; root.reset(); } } }
                }
                RowLayout { spacing: 12
                    Helmet { slug: root.desk.partnerInfo ? root.desk.partnerInfo.team.helmet : ""; size: 40 }
                    ColumnLayout { Layout.fillWidth: true; spacing: 0
                        Text { text: root.desk.partnerInfo ? root.desk.partnerInfo.team.name : ""; color: Theme.ink; font.bold: true; font.pixelSize: 16 }
                        Text { text: root.desk.partnerInfo ? root.desk.partnerInfo.team.record + " · " + root.desk.partnerInfo.mood + " · " + root.desk.partnerInfo.roster + " players" : ""; color: Theme.inkSubtle; font.pixelSize: 12 }
                        Text { text: root.desk.partnerInfo ? root.desk.partnerInfo.blurb : ""; color: Theme.inkSubtle; font.pixelSize: 12 }
                    }
                    ColumnLayout { spacing: 0
                        Text { text: "Their cash"; color: Theme.inkSubtle; font.pixelSize: 11; Layout.alignment: Qt.AlignRight }
                        Text { text: root.desk.partnerInfo ? root.desk.partnerInfo.cash : ""; color: Theme.ink; font.bold: true; Layout.alignment: Qt.AlignRight }
                    }
                }
            }
            Flow { Layout.fillWidth: true; spacing: 5
                Text { text: "Position"; color: Theme.inkSubtle; font.pixelSize: 12; height: 28; verticalAlignment: Text.AlignVCenter }
                Repeater { model: ["All", "QB", "RB", "WR", "TE", "OL", "DL", "LB", "DB", "K", "P"]; Chip { text: modelData; selected: root.pos === modelData; implicitHeight: 28; font.pixelSize: 12; onClicked: root.pos = modelData } }
            }

            GridLayout {
                Layout.fillWidth: true
                columns: root.width >= 1100 ? 3 : (root.width >= 700 ? 2 : 1)
                columnSpacing: 12; rowSpacing: 12
                Card {
                    title: "You get from " + (root.desk.partnerInfo ? root.desk.partnerInfo.team.town : "")
                    Layout.alignment: Qt.AlignTop; Layout.preferredWidth: 1
                    AssetList { Layout.fillWidth: true; players: root.desk.getPlayers || []; picks: root.desk.getPicks || []; chosenPlayers: root.getPlayers; chosenPicks: root.getPicks
                        onPlayerToggled: function(id, on) { root.getPlayers = root.toggled(root.getPlayers, id, on) }
                        onPickToggled: function(k, on) { root.getPicks = root.toggled(root.getPicks, k, on) } }
                }
                Card {
                    title: "You send"
                    Layout.alignment: Qt.AlignTop; Layout.preferredWidth: 1
                    AssetList { Layout.fillWidth: true; players: root.desk.givePlayers || []; picks: root.desk.givePicks || []; chosenPlayers: root.givePlayers; chosenPicks: root.givePicks
                        onPlayerToggled: function(id, on) { root.givePlayers = root.toggled(root.givePlayers, id, on) }
                        onPickToggled: function(k, on) { root.givePicks = root.toggled(root.givePicks, k, on) } }
                }
                ColumnLayout {
                    Layout.alignment: Qt.AlignTop; Layout.preferredWidth: 1; Layout.fillWidth: true
                    Layout.columnSpan: root.width >= 1100 ? 1 : (root.width >= 700 ? 2 : 1)
                    spacing: 12
                    Card {
                        title: "Cash"
                        RowLayout { spacing: 6
                            Chip { text: "Send cash"; selected: root.sendCash; onClicked: root.sendCash = true }
                            Chip { text: "Ask for cash"; selected: !root.sendCash; onClicked: root.sendCash = false }
                        }
                        RowLayout { Layout.fillWidth: true
                            PrimaryButton { text: "−"; ghost: true; implicitHeight: 36; implicitWidth: 40; leftPadding: 4; rightPadding: 4; enabled: root.cash > 0; onClicked: root.cash = Math.max(0, root.cash - 500) }
                            Text { text: Theme.money(root.cash); color: Theme.ink; font.pixelSize: 22; font.bold: true; horizontalAlignment: Text.AlignHCenter; Layout.fillWidth: true }
                            PrimaryButton { text: "+"; ghost: true; implicitHeight: 36; implicitWidth: 40; leftPadding: 4; rightPadding: 4; onClicked: root.cash += 500 }
                        }
                        Text { text: root.sendCash ? "Cash is worth the same to everyone, but a broke club values it more." : "They only give up cash they can spare above their reserve."; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                        Text { text: "Your cash: " + (root.desk.myCash || ""); color: Theme.inkMuted; font.pixelSize: 12 }
                    }
                    Card {
                        title: "Their answer"
                        Text { visible: !!root.verdict.empty; text: "Tick the players and picks you want and what you will send. They answer as you build the deal."; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                        ColumnLayout {
                            visible: !root.verdict.empty
                            Layout.fillWidth: true; spacing: 8
                            readonly property string tone: root.verdict.blocked ? "bad" : (root.verdict.accepted ? "good" : (root.verdict.mood === "close" ? "warn" : "bad"))
                            Rectangle {
                                Layout.fillWidth: true; implicitHeight: av.implicitHeight + 20; radius: Theme.radiusChip + 2
                                color: parent.tone === "good" ? Theme.goodBg : (parent.tone === "warn" ? Theme.warnBg : Theme.badBg)
                                ColumnLayout { id: av; anchors.fill: parent; anchors.margins: 10; spacing: 2
                                    Text { text: root.verdict.blocked ? "Cannot do this" : (root.verdict.accepted ? "They will do it" : (root.verdict.mood === "close" ? "Close" : "No deal")); font.bold: true; font.pixelSize: 15
                                        color: parent.parent.parent.tone === "good" ? Theme.goodFg : (parent.parent.parent.tone === "warn" ? Theme.warnFg : Theme.badFg) }
                                    Text { text: root.verdict.blocked || root.verdict.line || ""; wrapMode: Text.WordWrap; Layout.fillWidth: true; font.pixelSize: 13
                                        color: parent.parent.parent.tone === "good" ? Theme.goodFg : (parent.parent.parent.tone === "warn" ? Theme.warnFg : Theme.badFg) }
                                }
                            }
                            Rectangle { visible: !root.verdict.blocked; Layout.fillWidth: true; height: 8; radius: 4; color: Theme.surfaceAlt
                                Rectangle { width: parent.width * (root.verdict.ratio || 0); height: parent.height; radius: 4; color: root.verdict.accepted ? Theme.primary : Theme.warnFg } }
                            Text { visible: !root.verdict.blocked; text: root.verdict.accepted ? "They get more than they give up." : "They need to receive more value than they give up."; color: Theme.inkSubtle; font.pixelSize: 12 }
                            Repeater { model: root.verdict.notes || []; Text { text: modelData; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true } }
                            RowLayout { visible: !!root.verdict.ask; Layout.fillWidth: true
                                Text { text: "They would take: " + (root.verdict.ask ? root.verdict.ask.label : ""); color: Theme.ink; font.bold: true; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                                PrimaryButton { text: "Add it"; ghost: true; implicitHeight: 30; onClicked: root.addAsk(root.verdict.ask) }
                            }
                            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.divider }
                            Text { text: "THE MONEY"; color: Theme.inkSubtle; font.pixelSize: 11; font.bold: true; font.letterSpacing: 1 }
                            GridLayout { Layout.fillWidth: true; columns: 2; columnSpacing: 8; rowSpacing: 8
                                StatTile { label: "Pay per game"; value: root.verdict.book ? root.verdict.book.payroll : ""; accent: root.verdict.book && root.verdict.book.payrollDelta > 0 ? Theme.badFg : Theme.goodFg }
                                StatTile { label: "Contracts"; value: root.verdict.book ? root.verdict.book.contracts : "" }
                                StatTile { label: "Cash"; value: root.verdict.book ? root.verdict.book.cash : ""; accent: root.verdict.book && root.verdict.book.cashNeg ? Theme.badFg : Theme.ink }
                                StatTile { label: "Roster after"; value: root.verdict.book ? String(root.verdict.book.roster) : "" }
                            }
                            Repeater { model: root.verdict.book ? root.verdict.book.lines : []
                                RowLayout { Layout.fillWidth: true; spacing: 6
                                    Text { text: modelData.tag; color: modelData.tag === "IN" ? Theme.goodFg : Theme.badFg; font.bold: true; font.pixelSize: 11; Layout.preferredWidth: 28; Layout.alignment: Qt.AlignTop }
                                    ColumnLayout { Layout.fillWidth: true; spacing: 0
                                        Text { text: modelData.pos + " " + modelData.name + " (" + modelData.ovr + ")"; color: Theme.ink; font.bold: true; font.pixelSize: 13 }
                                        Text { text: modelData.text; color: Theme.inkSubtle; font.pixelSize: 11; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                                    }
                                } }
                            Repeater { model: root.verdict.book ? root.verdict.book.warnings : []; Text { text: "⚠ " + modelData; color: Theme.warnFg; font.pixelSize: 12; font.bold: true; wrapMode: Text.WordWrap; Layout.fillWidth: true } }
                            RowLayout { Layout.fillWidth: true; spacing: 8
                                PrimaryButton { text: "Make the trade"; Layout.fillWidth: true; enabled: !!root.verdict.accepted; onClicked: root.propose() }
                                PrimaryButton { text: "Clear"; ghost: true; onClicked: root.reset() }
                            }
                        }
                        Text { visible: root.notice !== ""; text: root.notice; color: Theme.goodFg; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    }
                }
            }
            Item { Layout.preferredHeight: 10 }
        }
    }

    Popup {
        id: infoDlg
        parent: Overlay.overlay; anchors.centerIn: parent; modal: true
        width: Math.min(parent ? parent.width - 32 : 420, 420)
        padding: 16
        background: Rectangle { color: Theme.surface; radius: Theme.radiusCard + 2; border.color: Theme.outline }
        Overlay.modal: Rectangle { color: "#aa0b0b12" }
        contentItem: ColumnLayout {
            spacing: 8
            Text { text: (root.infoLine.pos || "") + " " + (root.infoLine.name || "") + " · OVR " + (root.infoLine.ovr || ""); color: Theme.ink; font.bold: true; font.pixelSize: 16 }
            Text { text: "Age " + (root.infoLine.age || "") + " · " + (root.infoLine.hometown || "") + " · " + (root.infoLine.job || ""); color: Theme.inkSubtle; font.pixelSize: 12 }
            Text { visible: !root.infoLine.ppg; text: "No contract on file."; color: Theme.inkMuted }
            GridLayout { visible: !!root.infoLine.ppg; Layout.fillWidth: true; columns: 2; columnSpacing: 8; rowSpacing: 8
                StatTile { label: "Per game"; value: root.infoLine.ppg || "" }
                StatTile { label: "Commute at " + (root.infoLine.toMe ? "your club" : (root.infoLine.toName || "")); value: root.infoLine.newCommute || "" }
                StatTile { label: "Years left"; value: String(root.infoLine.years || "") }
                StatTile { label: "Market ask (3 yr)"; value: root.infoLine.market || "" }
                StatTile { label: "Against market"; value: (root.infoLine.surplus >= 0 ? "+" : "−") + (root.infoLine.surplusText || ""); accent: root.infoLine.surplus >= 0 ? Theme.goodFg : Theme.badFg }
                StatTile { label: "To cut him"; value: root.infoLine.release || "" }
            }
            PrimaryButton { text: "Close"; ghost: true; Layout.alignment: Qt.AlignRight; onClicked: infoDlg.close() }
        }
    }
}
