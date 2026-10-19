import QtQuick

// Team helmet from the shared art folder. Falls back to nothing (no broken-image box) if the file is missing.
Item {
    id: root
    property string slug: ""
    property int size: 48
    implicitWidth: size
    implicitHeight: size
    Image {
        anchors.fill: parent
        source: root.slug === "" || !Store.game.assetsUrl ? "" : Store.game.assetsUrl + "artwork/helmets/" + root.slug + ".png"
        sourceSize.width: root.size * 2
        sourceSize.height: root.size * 2
        fillMode: Image.PreserveAspectFit
        asynchronous: true
        smooth: true
        visible: status === Image.Ready
    }
}
