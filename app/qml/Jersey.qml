import QtQuick

// A team jersey (back view) with the player's number. Art lives in assets/artwork/jerseys as `slug_inkcolor.png`;
// a team with no art gets a drawn jersey.
Item {
    id: root
    property int number: 0
    property string team: ""        // helmet-style slug of the team nickname
    property string lastName: ""
    property int size: 96

    readonly property string file: Store.jerseyArt[team] || ""
    readonly property string colorName: file === "" ? "" : file.substring(file.lastIndexOf("_") + 1, file.length - 4)
    readonly property var inks: ({ white: "#ffffff", cream: "#f3e8c8", navy: "#14284b", darkgreen: "#14432a", black: "#111111" })
    readonly property color ink: inks[colorName] || "#ffffff"
    readonly property bool hasName: lastName !== ""
    implicitWidth: size
    implicitHeight: size

    Image {
        id: art
        anchors.fill: parent
        visible: root.file !== "" && status === Image.Ready
        source: root.file === "" ? "" : Store.game.assetsUrl + "artwork/jerseys/" + root.file
        sourceSize.width: root.size * 3
        sourceSize.height: root.size * 3
        fillMode: Image.PreserveAspectFit
        asynchronous: true
        smooth: true
    }
    Text {
        visible: art.visible && root.hasName
        anchors.horizontalCenter: parent.horizontalCenter
        y: parent.height * 0.24 - height / 2
        width: root.size * 0.56
        horizontalAlignment: Text.AlignHCenter
        text: root.lastName.toUpperCase()
        color: root.ink
        font.pixelSize: root.size * 0.09
        font.weight: Font.Black
        fontSizeMode: Text.Fit
        minimumPixelSize: 4
        elide: Text.ElideNone
    }
    Text {
        visible: art.visible
        anchors.horizontalCenter: parent.horizontalCenter
        y: parent.height * (root.hasName ? 0.51 : 0.44) - height / 2
        text: root.number
        color: root.ink
        font.pixelSize: root.size * (root.number > 9 ? 0.36 : 0.42)
        font.weight: Font.Black
    }

    // fallback: a drawn jersey in a color keyed off the team
    Canvas {
        id: drawn
        anchors.fill: parent
        visible: !art.visible && (root.file === "" || art.status === Image.Error)
        readonly property color body: { var h = 7; for (var i = 0; i < root.team.length; i++) h = (h * 31 + root.team.charCodeAt(i)) & 0x7fffffff; return ["#b3261e", "#1f3a68", "#1e5b3a", "#5b3a8c", "#d97706", "#0f766e", "#2b2b2b", "#1f6fb2", "#7a1f3d", "#c9a227", "#55626b", "#8a5a2b"][h % 12]; }
        onBodyChanged: requestPaint()
        onWidthChanged: requestPaint()
        onPaint: {
            var ctx = getContext("2d"), w = width, h = height;
            ctx.reset();
            var pts = [[.28, .04], [.40, .04], [.50, .20], [.60, .04], [.72, .04], [.98, .24], [.86, .40], [.76, .32], [.76, .96], [.24, .96], [.24, .32], [.14, .40], [.02, .24]];
            ctx.beginPath();
            ctx.moveTo(pts[0][0] * w, pts[0][1] * h);
            for (var i = 1; i < pts.length; i++) ctx.lineTo(pts[i][0] * w, pts[i][1] * h);
            ctx.closePath();
            ctx.fillStyle = body; ctx.fill();
            ctx.lineWidth = w * 0.03; ctx.strokeStyle = "rgba(0,0,0,0.45)"; ctx.stroke();
        }
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            y: parent.height * 0.56 - height / 2
            text: root.number
            color: "#ffffff"
            font.pixelSize: root.size * (root.number > 9 ? 0.34 : 0.40)
            font.weight: Font.Black
        }
    }
}
