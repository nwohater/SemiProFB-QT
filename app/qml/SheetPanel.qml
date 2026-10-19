import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// The green-bar books: profit and loss for the season so far, then every game's money in and out.
ScrollView {
    id: sv
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.financeSheetJson()); }
    readonly property int wk: 40; readonly property int loc: 34; readonly property int att: 58; readonly property int pct: 52
    readonly property int col: 76; readonly property int tot: 92
    readonly property int tableW: wk + loc + att + pct + col * ((vm.revHeads || []).length + (vm.expHeads || []).length) + tot * 5
    readonly property color bar: "#1c2a3a"
    readonly property color paper: "#1a1f31"
    function tone(t) { return t === "good" ? Theme.goodFg : (t === "bad" ? Theme.badFg : Theme.ink); }
    contentWidth: availableWidth
    clip: true

    component Cell: Text {
        property int w: 60
        property bool head: false
        property bool leftAlign: false
        width: w; height: parent ? parent.height : 30
        horizontalAlignment: leftAlign ? Text.AlignLeft : Text.AlignRight
        verticalAlignment: Text.AlignVCenter
        rightPadding: 6; leftPadding: 6
        font.family: "monospace"; font.pixelSize: head ? 12 : 13; font.bold: head
        color: head ? Theme.ink : Theme.inkMuted
        elide: Text.ElideRight
    }
    component SheetRow: Rectangle {
        property var r
        property bool header: false
        property bool total: false
        property int idx: 0
        width: sv.tableW; height: header ? 34 : 30
        color: header ? Theme.primaryDark : (total ? Theme.surfaceHi : (idx % 2 === 0 ? sv.paper : sv.bar))
        Row {
            anchors.fill: parent
            Cell { w: sv.wk; leftAlign: true; head: header; text: header ? "Wk" : r.week; font.bold: total }
            Cell { w: sv.loc; head: header; text: header ? "H/A" : r.ha }
            Cell { w: sv.att; head: header; text: header ? "Crowd" : r.crowd }
            Cell { w: sv.pct; head: header; text: header ? "%Full" : r.full }
            Repeater { model: header ? sv.vm.revHeads : r.rev; Cell { w: sv.col; head: header; text: modelData; color: header ? Theme.ink : Theme.goodFg } }
            Cell { w: sv.tot; head: header; text: header ? "REVENUE" : r.revT; color: header ? Theme.ink : Theme.goodFg; font.bold: true }
            Repeater { model: header ? sv.vm.expHeads : r.exp; Cell { w: sv.col; head: header; text: modelData; color: header ? Theme.ink : Theme.badFg } }
            Cell { w: sv.tot; head: header; text: header ? "EXPENSE" : r.expT; color: header ? Theme.ink : Theme.badFg; font.bold: true }
            Cell { w: sv.tot; head: header; text: header ? "NET" : r.net; color: header ? Theme.ink : (r.netNeg ? Theme.badFg : Theme.goodFg); font.bold: true }
            Cell { w: sv.tot; head: header; text: header ? "YTD NET" : r.ytd; color: header ? Theme.ink : (r.ytdNeg ? Theme.badFg : Theme.goodFg); font.bold: true }
            Cell { w: sv.tot; head: header; text: header ? "CASH" : r.cash; color: header ? Theme.ink : (r.cashNeg ? Theme.badFg : Theme.ink) }
        }
    }

    ColumnLayout {
        width: sv.availableWidth - 8
        spacing: 10
        Card {
            title: "Profit & loss, season to date"
            trailing: Pill { text: sv.vm.profit ? "PROFIT" : "LOSS"; strong: true; fg: sv.vm.profit ? Theme.goodFg : Theme.badFg; bg: sv.vm.profit ? Theme.goodBg : Theme.badBg }
            Repeater {
                model: sv.vm.pl || []
                Rectangle {
                    Layout.fillWidth: true; implicitHeight: 32
                    color: modelData.rule ? Theme.surfaceHi : (index % 2 === 0 ? sv.paper : sv.bar)
                    border.color: modelData.rule ? Theme.ink : "transparent"
                    RowLayout {
                        anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10
                        Text { text: modelData.label; color: Theme.inkMuted; font.family: "monospace"; font.pixelSize: 13; font.bold: modelData.bold; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { text: modelData.value; color: sv.tone(modelData.tone); font.family: "monospace"; font.pixelSize: 13; font.bold: modelData.bold }
                    }
                }
            }
        }
        Card {
            title: "Season " + sv.vm.season + " books"
            trailing: Text { text: sv.vm.games + " game" + (sv.vm.games === 1 ? "" : "s"); color: Theme.inkSubtle; font.pixelSize: 12 }
            Text { text: "Every dollar in and out, game by game. Revenue in green, expenses in red, (parentheses) are negative. Scroll sideways."; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            Text { visible: sv.vm.games === 0; text: "Play a game and the sheet fills in."; color: Theme.inkMuted }
            Flickable {
                visible: sv.vm.games > 0
                Layout.fillWidth: true
                Layout.preferredHeight: tbl.implicitHeight
                contentWidth: sv.tableW; contentHeight: tbl.implicitHeight
                flickableDirection: Flickable.HorizontalFlick
                boundsBehavior: Flickable.StopAtBounds
                clip: true
                ScrollBar.horizontal: ScrollBar {}
                Column {
                    id: tbl
                    width: sv.tableW
                    SheetRow { header: true }
                    Repeater { model: sv.vm.rows || []; SheetRow { r: modelData; idx: index } }
                    SheetRow { visible: sv.vm.games > 0; r: sv.vm.totals; total: true }
                }
            }
            Text { visible: sv.vm.games > 0; text: "Total row: crowd is the average per home game; cash is where you are now."; color: Theme.inkSubtle; font.pixelSize: 12 }
        }
        Card {
            visible: (sv.vm.other || []).length > 0
            title: "Other entries"
            Repeater {
                model: sv.vm.other || []
                RowLayout {
                    Layout.fillWidth: true; spacing: 8
                    Text { text: "Wk " + modelData.week; color: Theme.inkFaint; font.family: "monospace"; font.pixelSize: 13; Layout.preferredWidth: 48 }
                    Text { text: modelData.text; color: Theme.inkMuted; font.family: "monospace"; font.pixelSize: 13; Layout.fillWidth: true; elide: Text.ElideRight }
                    Text { text: modelData.amount; color: modelData.neg ? Theme.badFg : Theme.goodFg; font.family: "monospace"; font.pixelSize: 13; font.bold: true }
                }
            }
        }
        Item { Layout.preferredHeight: 8 }
    }
}
