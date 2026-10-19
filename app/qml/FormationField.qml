import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes

// "Who is on the field" for the chosen package, with weak spots dashed. Click a dot to open the player.
Rectangle {
    id: root
    property bool offense: true
    property var slots: ({})          // position label -> how many start
    property var groups: []           // the unit's depth groups (key, rows[{id, rate}])
    property bool fits: true
    property string pkg: ""
    readonly property real lineY: 0.2
    readonly property int d: 30
    readonly property int fieldH: 210
    readonly property color turfA: offense ? "#2d6b45" : "#2a507f"
    readonly property color turfB: offense ? "#357a50" : "#325b8e"
    readonly property color ring: offense ? "#1e5b3a" : "#1f4e79"

    function rowsOf(pos) { for (var i = 0; i < groups.length; i++) if (groups[i].key === pos) return groups[i].rows; return []; }
    function sp(pos, idx, label, x, y) { return { pos: pos, idx: idx, label: label, x: x, y: y }; }
    readonly property var spots: {
        var out = [];
        if (offense) {
            var ol = [["OT", 0], ["OG", 0], ["C", 0], ["OG", 1], ["OT", 1]];
            for (var i = 0; i < 5; i++) out.push(sp(ol[i][0], ol[i][1], "OL" + (i + 1), 0.34 + i * 0.08, lineY));
            var te = slots["TE"] || 0;
            if (te >= 1) out.push(sp("TE", 0, "TE1", 0.74, lineY));
            if (te >= 2) out.push(sp("TE", 1, "TE2", 0.26, lineY));
            var wr = slots["WR"] || 0;
            var wrXY = [[0.06, lineY], [0.94, lineY], [0.84, lineY + 0.2], [0.16, lineY + 0.2]];
            for (var j = 0; j < wr; j++) out.push(sp("WR", j, "WR" + (j + 1), wrXY[j][0], wrXY[j][1]));
            out.push(sp("QB", 0, "QB1", 0.5, lineY + 0.24));
            if ((slots["FB"] || 0) > 0) {
                var split = wr === 2 && te === 1;
                out.push(sp("FB", 0, "FB1", split ? 0.42 : 0.5, lineY + (split ? 0.46 : 0.4)));
                out.push(sp("RB", 0, "RB1", split ? 0.58 : 0.5, lineY + (split ? 0.46 : 0.56)));
            } else out.push(sp("RB", 0, "RB1", 0.5, lineY + 0.46));
        } else {
            var xs = { DE: { 2: [0.3, 0.7] }, DT: { 1: [0.5], 2: [0.43, 0.57] }, MLB: { 1: [0.5], 2: [0.38, 0.62] }, OLB: { 1: [0.16], 2: [0.16, 0.84] }, CB: { 2: [0.03, 0.97], 3: [0.03, 0.97, 0.72] }, S: { 1: [0.5], 2: [0.36, 0.64] } };
            var ys = { DE: [0.2, 0.2], DT: [0.2, 0.2], MLB: [0.56, 0.56], OLB: [0.5, 0.5], CB: [0.34, 0.34, 0.62], S: [0.88, 0.88] };
            var order = ["DE", "DT", "MLB", "OLB", "CB", "S"];
            for (var k = 0; k < order.length; k++) {
                var p = order[k], n = slots[p] || 0;
                var list = xs[p][n];
                if (!list) continue;
                for (var m = 0; m < n; m++) out.push(sp(p, m, p + (m + 1), list[m], ys[p][m]));
            }
        }
        return out;
    }
    // each spot's player (or none) and his rating at the spot
    readonly property var filled: spots.map(function(s) { var r = rowsOf(s.pos); var row = s.idx < r.length ? r[s.idx] : null; return { spot: s, id: row ? row.id : null, ovr: row ? row.rate : null }; })
    readonly property real avg: { var v = filled.filter(function(f) { return f.ovr !== null; }).map(function(f) { return f.ovr; }); return v.length ? v.reduce(function(a, b) { return a + b; }, 0) / v.length : 0; }
    function weak(o) { return o === null || o <= avg - 4; }
    readonly property var weakLabels: filled.filter(function(f) { return weak(f.ovr); }).map(function(f) { return f.spot.label; })
    function joinList(l) { return l.length <= 1 ? l.join("") : l.slice(0, -1).join(", ") + " and " + l[l.length - 1]; }

    Layout.fillWidth: true
    Layout.maximumWidth: 760
    Layout.alignment: Qt.AlignHCenter
    implicitHeight: col.implicitHeight
    radius: Theme.radiusCard
    color: turfA
    clip: true
    ColumnLayout {
        id: col
        width: parent.width
        spacing: 0
        Text { text: root.pkg.toUpperCase() + " · CLICK A PLAYER FOR HIS BIO"; color: "#eaf2ec"; font.pixelSize: 11; font.bold: true; font.letterSpacing: 0.9; leftPadding: 14; topPadding: 12; bottomPadding: 4 }
        Item {
            id: field
            Layout.fillWidth: true
            Layout.preferredHeight: root.fieldH
            Repeater { model: Math.ceil(root.fieldH / 28); Rectangle { y: index * 28; width: field.width; height: 28; color: index % 2 === 0 ? root.turfA : root.turfB } }
            Repeater {
                model: Math.ceil(field.width / 10)
                Rectangle { x: index * 10; y: root.lineY * (root.fieldH - 40) + root.d / 2 + 6; width: 5; height: 2; color: "#bfffffff" }
            }
            Repeater {
                model: root.filled
                Item {
                    readonly property bool isWeak: root.weak(modelData.ovr)
                    x: Math.max(0, Math.min(field.width - 46, modelData.spot.x * (field.width - root.d) - (46 - root.d) / 2))
                    y: modelData.spot.y * (root.fieldH - 40)
                    width: 46; height: root.d + 20
                    Rectangle {
                        x: 8; y: 4; width: root.d; height: root.d; radius: root.d / 2
                        color: isWeak ? Theme.warnBg : "white"
                        border.width: isWeak ? 0 : 2; border.color: root.ring
                        Shape {
                            visible: isWeak
                            anchors.fill: parent
                            ShapePath {
                                strokeColor: "#e0a526"; strokeWidth: 2; fillColor: "transparent"
                                strokeStyle: ShapePath.DashLine; dashPattern: [2, 2]
                                PathAngleArc { centerX: root.d / 2; centerY: root.d / 2; radiusX: root.d / 2 - 1; radiusY: root.d / 2 - 1; startAngle: 0; sweepAngle: 360 }
                            }
                        }
                        Text { anchors.centerIn: parent; text: modelData.ovr !== null ? modelData.ovr : "-"; color: isWeak ? Theme.warnFg : "#14283d"; font.pixelSize: 13; font.bold: true }
                    }
                    Text { y: root.d + 6; width: 46; horizontalAlignment: Text.AlignHCenter; text: modelData.spot.label; color: "#eaf2ec"; font.pixelSize: 10; font.bold: true }
                    HoverHandler { cursorShape: modelData.id !== null ? Qt.PointingHandCursor : Qt.ArrowCursor }
                    TapHandler { enabled: modelData.id !== null; onTapped: Store.openPlayer(modelData.id) }
                }
            }
        }
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: foot.implicitHeight + 22
            color: "#38000000"
            ColumnLayout {
                id: foot
                anchors.fill: parent; anchors.margins: 12; anchors.topMargin: 10; spacing: 4
                Text { text: root.weakLabels.length === 0 ? "No weak spots." : "Dashed means a weak spot: " + root.joinList(root.weakLabels) + "."; color: "#eaf2ec"; font.pixelSize: 14; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                Text { text: root.fits ? "Matches your " + (root.offense ? "offense" : "defense") + " strategy." : "Not your strategy formation; for planning only."; color: "#c9d8cf"; font.pixelSize: 12 }
            }
        }
    }
}
