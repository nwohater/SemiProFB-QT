import QtQuick
import QtQuick.Controls

// A little map of the chosen state: its outline, the league's towns as dots (your division in the accent colour) and your club ringed.
// Outlines and town coordinates come from the Flutter app. Tap a dot to pick that town.
Item {
    id: root
    property string region: "ca"
    property var towns: []
    property string selectedId: ""
    property string division: ""
    signal picked(string id)

    // lon/lat box and the cos(lat) squeeze, per state
    readonly property var frames: ({
        ca: { x0: -124.6, x1: -114.0, y0: 32.4, y1: 42.1, k: 0.82 },
        tx: { x0: -107.0, x1: -93.3, y0: 25.6, y1: 36.7, k: 0.86 },
        fl: { x0: -88.0, x1: -79.8, y0: 24.8, y1: 31.2, k: 0.88 },
        pa: { x0: -81.0, x1: -74.4, y0: 39.5, y1: 42.4, k: 0.77 }
    })
    readonly property var outlines: ({"ca": [[-124.2, 42.0], [-120.0, 42.0], [-120.0, 39.0], [-114.6, 35.0], [-114.7, 32.7], [-117.1, 32.5], [-118.4, 33.8], [-119.2, 34.1], [-120.6, 34.5], [-121.9, 36.6], [-122.5, 37.8], [-123.8, 39.4], [-124.4, 40.4], [-124.1, 41.8]], "tx": [[-103.0, 36.5], [-100.0, 36.5], [-100.0, 34.5], [-99.3, 34.2], [-97.2, 33.8], [-96.5, 33.8], [-94.0, 33.55], [-94.05, 31.0], [-93.55, 29.8], [-94.7, 29.35], [-97.0, 27.8], [-97.15, 25.95], [-99.1, 26.4], [-99.5, 27.5], [-100.7, 29.1], [-101.4, 29.8], [-102.4, 29.8], [-103.1, 29.0], [-104.5, 29.7], [-104.9, 30.6], [-106.5, 31.8], [-106.6, 32.0], [-103.06, 32.0]], "fl": [[-87.6, 31.0], [-85.0, 31.0], [-84.9, 30.7], [-82.2, 30.55], [-82.0, 30.4], [-81.45, 30.7], [-81.3, 29.9], [-80.55, 28.4], [-80.05, 26.8], [-80.1, 25.7], [-80.4, 25.2], [-81.1, 25.15], [-81.8, 26.0], [-82.2, 26.9], [-82.8, 27.9], [-82.7, 28.9], [-83.7, 29.9], [-84.3, 30.05], [-85.3, 29.65], [-86.5, 30.4], [-87.6, 30.3]], "pa": [[-80.52, 41.98], [-80.1, 42.17], [-79.76, 42.27], [-79.76, 42.0], [-75.36, 42.0], [-75.0, 41.85], [-74.7, 41.35], [-75.1, 40.99], [-75.2, 40.6], [-74.95, 40.2], [-75.4, 39.8], [-75.78, 39.72], [-80.52, 39.72], [-80.52, 40.64]]})
    readonly property var geo: ({"fresno": [-119.79, 36.74], "visalia": [-119.29, 36.33], "stockton": [-121.29, 37.96], "chico": [-121.84, 39.73], "slo": [-120.66, 35.28], "monterey": [-121.89, 36.6], "thousand_oaks": [-118.84, 34.17], "paso_robles": [-120.69, 35.63], "bakersfield": [-119.02, 35.37], "rancho_cucamonga": [-117.59, 34.1], "pasadena": [-118.14, 34.15], "san_bernardino": [-117.29, 34.11], "modesto": [-121.0, 37.64], "redding": [-122.39, 40.59], "merced": [-120.48, 37.3], "yuba_city": [-121.62, 39.14], "lodi": [-121.27, 38.13], "santa_maria": [-120.44, 34.95], "santa_barbara": [-119.7, 34.42], "salinas": [-121.66, 36.68], "ventura": [-119.29, 34.28], "oxnard": [-119.18, 34.2], "riverside": [-117.4, 33.95], "lancaster": [-118.15, 34.7], "palmdale": [-118.12, 34.58], "santa_clarita": [-118.54, 34.39], "victorville": [-117.29, 34.54], "lubbock": [-101.86, 33.58], "amarillo": [-101.83, 35.22], "abilene": [-99.73, 32.45], "wichita_falls": [-98.49, 33.91], "corpus_christi": [-97.4, 27.8], "beaumont": [-94.13, 30.08], "galveston": [-94.8, 29.3], "victoria": [-97.0, 28.81], "waco": [-97.15, 31.55], "killeen": [-97.73, 31.12], "college_station": [-96.33, 30.63], "tyler": [-95.3, 32.35], "midland": [-102.08, 32.0], "odessa": [-102.37, 31.85], "san_angelo": [-100.44, 31.46], "el_paso": [-106.49, 31.76], "denton": [-97.13, 33.21], "port_arthur": [-93.93, 29.9], "brownsville": [-97.5, 25.9], "texas_city": [-94.9, 29.38], "baytown": [-94.98, 29.74], "lake_jackson": [-95.43, 29.03], "temple": [-97.34, 31.1], "round_rock": [-97.68, 30.51], "san_marcos": [-97.94, 29.88], "longview": [-94.74, 32.5], "georgetown": [-97.68, 30.63], "tallahassee": [-84.28, 30.44], "gainesville": [-82.32, 29.65], "pensacola": [-87.22, 30.42], "sarasota": [-82.53, 27.34], "fort_myers": [-81.87, 26.64], "fort_lauderdale": [-80.14, 26.12], "west_palm_beach": [-80.05, 26.72], "panama_city": [-85.66, 30.16], "ocala": [-82.14, 29.19], "st_augustine": [-81.31, 29.9], "lake_city": [-82.64, 30.19], "palm_coast": [-81.21, 29.58], "clearwater": [-82.8, 27.97], "lakeland": [-81.95, 28.04], "naples": [-81.79, 26.14], "bradenton": [-82.57, 27.5], "cape_coral": [-81.95, 26.56], "hialeah": [-80.28, 25.86], "daytona_beach": [-81.02, 29.21], "melbourne": [-80.61, 28.08], "port_st_lucie": [-80.35, 27.27], "kissimmee": [-81.41, 28.29], "fort_walton_beach": [-86.62, 30.42], "largo": [-82.79, 27.91], "punta_gorda": [-82.05, 26.93], "boynton_beach": [-80.07, 26.53], "deltona": [-81.26, 28.9], "erie": [-80.09, 42.13], "johnstown": [-78.92, 40.33], "altoona": [-78.39, 40.52], "state_college": [-77.86, 40.79], "harrisburg": [-76.88, 40.27], "williamsport": [-77.0, 41.24], "york": [-76.73, 39.96], "gettysburg": [-77.23, 39.83], "scranton": [-75.66, 41.41], "wilkes_barre": [-75.88, 41.25], "allentown": [-75.47, 40.6], "reading": [-75.93, 40.34], "new_castle": [-80.35, 41.0], "butler": [-79.9, 40.86], "greensburg": [-79.54, 40.3], "washington": [-80.25, 40.17], "meadville": [-80.15, 41.64], "lebanon": [-76.41, 40.34], "chambersburg": [-77.66, 39.94], "carlisle": [-77.19, 40.2], "sunbury": [-76.79, 40.86], "lock_haven": [-77.45, 41.14], "bethlehem": [-75.37, 40.63], "hazleton": [-75.97, 40.96], "pottsville": [-76.2, 40.69], "easton": [-75.22, 40.69], "stroudsburg": [-75.19, 40.99]})
    readonly property var fr: frames[region] || frames.ca

    function pt(lon, lat) {
        var f = fr, w = (f.x1 - f.x0) * f.k, h = f.y1 - f.y0;
        var sc = Math.min(width / w, height / h);
        return { x: (width - w * sc) / 2 + (lon - f.x0) * f.k * sc, y: (height - h * sc) / 2 + (f.y1 - lat) * sc };
    }
    function dots() {
        var out = [];
        for (var i = 0; i < towns.length; i++) {
            var g = geo[towns[i].id];
            if (g) { var p = pt(g[0], g[1]); out.push({ id: towns[i].id, name: towns[i].name, division: towns[i].division, x: p.x, y: p.y }); }
        }
        return out;
    }
    readonly property var dotList: { width; height; towns; region; return dots(); }

    Canvas {
        id: cv
        anchors.fill: parent
        onPaint: {
            var c = getContext("2d");
            c.reset();
            var o = root.outlines[root.region] || root.outlines.ca;
            c.beginPath();
            for (var i = 0; i < o.length; i++) { var p = root.pt(o[i][0], o[i][1]); if (i === 0) c.moveTo(p.x, p.y); else c.lineTo(p.x, p.y); }
            c.closePath();
            c.fillStyle = Theme.surfaceAlt; c.fill();
            c.lineJoin = "round"; c.lineWidth = 1.5; c.strokeStyle = Theme.inkFaint; c.stroke();
        }
        Connections { target: root; function onRegionChanged() { cv.requestPaint(); } function onWidthChanged() { cv.requestPaint(); } function onHeightChanged() { cv.requestPaint(); } }
        Component.onCompleted: requestPaint()
    }
    Repeater {
        model: root.dotList
        Item {
            readonly property bool sel: modelData.id === root.selectedId
            x: modelData.x; y: modelData.y
            Rectangle {
                width: sel ? 18 : 8; height: width; radius: width / 2
                x: -width / 2; y: -height / 2
                color: sel ? Theme.bg : (modelData.division === root.division ? Theme.primary : Qt.alpha(Theme.inkFaint, 0.8))
                border.width: sel ? 2.5 : 0; border.color: Theme.primary
                Rectangle { visible: sel; width: 8; height: 8; radius: 4; anchors.centerIn: parent; color: Theme.primary }
                Behavior on width { NumberAnimation { duration: 150 } }
            }
            MouseArea {
                x: -12; y: -12; width: 24; height: 24
                cursorShape: Qt.PointingHandCursor; hoverEnabled: true
                onClicked: root.picked(modelData.id)
                ToolTip.visible: containsMouse; ToolTip.text: modelData.name
            }
        }
    }
}
