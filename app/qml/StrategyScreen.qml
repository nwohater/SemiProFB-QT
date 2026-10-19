import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    property string tab: "Offense"
    readonly property var vm: { Store.game.rev; return JSON.parse(Store.game.strategyJson()); }
    readonly property var st: vm.strategy || ({ offense: {}, defense: {}, special_teams: {} })
    readonly property var o: st.offense
    readonly property var d: st.defense
    readonly property var s: st.special_teams
    function lean(v, lo, hi) { return v < 0.35 ? lo : (v > 0.65 ? hi : "Even"); }
    function update(group, key, value) {
        var n = JSON.parse(JSON.stringify(root.st));
        n[group][key] = value;
        Store.game.setStrategy(JSON.stringify(n));
    }

    component Fitted: ColumnLayout {
        property var info
        Layout.fillWidth: true
        spacing: 4
        Flow { Layout.fillWidth: true; spacing: 6; Repeater { model: info ? info.parts : []; FitChip { fit: modelData.fit; note: modelData.text } } }
        Text { visible: !!info; text: info ? info.reason : ""; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
    }
    component Tuner: ColumnLayout {
        id: tuner
        property string label
        property string readout
        property string lo
        property string hi
        property var info: null
        property alias value: sl.value
        property alias from: sl.from
        property alias to: sl.to
        signal moved(real v)
        signal released()
        Layout.fillWidth: true
        spacing: 2
        RowLayout { Layout.fillWidth: true
            Text { text: tuner.label; color: Theme.ink; font.bold: true; font.pixelSize: 14; Layout.fillWidth: true }
            FitChip { visible: !!tuner.info; fit: tuner.info ? tuner.info.fit : "Okay" }
        }
        Text { text: tuner.readout; color: Theme.inkSubtle; font.pixelSize: 12 }
        Slide { id: sl; Layout.fillWidth: true; from: 0; to: 1; stepSize: 0.05; onMoved: tuner.moved(value); onPressedChanged: if (!pressed) tuner.released() }
        RowLayout { Layout.fillWidth: true
            Text { text: tuner.lo; color: Theme.inkFaint; font.pixelSize: 11; Layout.fillWidth: true }
            Text { text: tuner.hi; color: Theme.inkFaint; font.pixelSize: 11 }
        }
        Fitted { info: tuner.info }
    }
    component Choice: Rectangle {
        id: choice
        property string title
        property var info: null
        property bool on: false
        signal picked()
        Layout.fillWidth: true
        implicitHeight: cc.implicitHeight + 24
        radius: Theme.radiusChip + 2
        color: on ? Theme.primaryTint : Theme.surface
        border.color: on ? Theme.primary : Theme.outline
        border.width: on ? 2 : 1
        TapHandler { onTapped: choice.picked() }
        HoverHandler { cursorShape: Qt.PointingHandCursor }
        ColumnLayout { id: cc; anchors.fill: parent; anchors.margins: 12; spacing: 6
            RowLayout { Layout.fillWidth: true
                Text { text: choice.title; color: Theme.ink; font.bold: true; font.pixelSize: 15; Layout.fillWidth: true }
                FitChip { fit: choice.info ? choice.info.fit : "Okay" }
            }
            Fitted { visible: choice.on; info: choice.info }
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
            ScreenTitle { text: "Game plan" }
            Card {
                visible: !!root.vm.opponent
                title: root.vm.opponent ? root.vm.opponent.team.name + " tendencies" : ""
                RowLayout { spacing: 12
                    Helmet { slug: root.vm.opponent ? root.vm.opponent.team.helmet : ""; size: 48 }
                    Text { text: root.vm.opponent ? root.vm.opponent.summary : ""; color: Theme.ink; font.pixelSize: 14; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                }
                Flow { Layout.fillWidth: true; spacing: 6; Repeater { model: root.vm.opponent ? root.vm.opponent.tips : []; Pill { text: modelData } } }
            }
            RowLayout { spacing: 6
                Repeater { model: ["Offense", "Defense", "Special teams"]; Chip { text: modelData; selected: root.tab === modelData; onClicked: root.tab = modelData } }
            }

            // ---------------- offense
            ColumnLayout {
                visible: root.tab === "Offense"
                Layout.fillWidth: true; spacing: 12
                Card { title: "Offensive scheme"
                    Repeater { model: root.vm.schemes || []; Choice { title: modelData.name; info: modelData; on: modelData.on; onPicked: root.update("offense", "scheme", modelData.id) } } }
                Card { title: "Tendencies"
                    Tuner { label: "Run / pass bias"; readout: Math.round((1 - root.o.runPassBias) * 100) + "% run, " + Math.round(root.o.runPassBias * 100) + "% pass"; lo: "All run"; hi: "All pass"; info: root.vm.runPass; value: root.o.runPassBias || 0; onMoved: function(v) { root.update("offense", "runPassBias", v) } onReleased: Store.game.save() }
                    Tuner { label: "Pass depth"; readout: root.lean(root.o.passDepth, "Short", "Deep"); lo: "Short"; hi: "Deep"; info: root.vm.depth; value: root.o.passDepth || 0; onMoved: function(v) { root.update("offense", "passDepth", v) } onReleased: Store.game.save() }
                    Tuner { label: "Tempo"; readout: root.lean(root.o.tempo, "Slower, burns clock", "Hurry-up"); lo: "Slow"; hi: "Hurry-up"; info: root.vm.tempo; value: root.o.tempo || 0; onMoved: function(v) { root.update("offense", "tempo", v) } onReleased: Store.game.save() }
                    Tuner { label: "4th-down aggression"; readout: root.lean(root.o.fourthDownAggression, "Punts a lot", "Goes for it"); lo: "Cautious"; hi: "Bold"; value: root.o.fourthDownAggression || 0; onMoved: function(v) { root.update("offense", "fourthDownAggression", v) } onReleased: Store.game.save() }
                    Tuner { label: "Play-action"; readout: Math.round((root.o.playAction || 0) * 100) + "%"; lo: "Never"; hi: "Often"; value: root.o.playAction || 0; onMoved: function(v) { root.update("offense", "playAction", v) } onReleased: Store.game.save() }
                }
                Card { title: "Pocket style"
                    trailing: FitChip { fit: root.vm.pocket ? root.vm.pocket.fit : "Okay" }
                    RowLayout { spacing: 6
                        Repeater { model: [["pocket", "Pocket"], ["rolling", "Rolling"], ["scramble", "Scramble"]]; Chip { text: modelData[1]; selected: root.o.pocket === modelData[0]; onClicked: { root.update("offense", "pocket", modelData[0]); Store.game.save(); } } }
                    }
                    Text { text: root.vm.pocket ? root.vm.pocket.reason : ""; color: Theme.inkSubtle; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                }
            }

            // ---------------- defense
            ColumnLayout {
                visible: root.tab === "Defense"
                Layout.fillWidth: true; spacing: 12
                Card { title: "Base front"
                    Repeater { model: root.vm.fronts || []; Choice { title: modelData.name; info: modelData; on: modelData.on; onPicked: { root.update("defense", "front", modelData.id); Store.game.save(); } } } }
                Card { title: "Coverage"
                    RowLayout { spacing: 6
                        Repeater { model: [["man", "Man"], ["zone", "Zone"], ["mix", "Mix"]]; Chip { text: modelData[1]; selected: root.d.coverage === modelData[0]; onClicked: { root.update("defense", "coverage", modelData[0]); Store.game.save(); } } }
                    }
                }
                Card { title: "Aggression"
                    Tuner { label: "Blitz frequency"; readout: Math.round((root.d.blitzFrequency || 0) * 100) + "% of dropbacks"; lo: "Never"; hi: "Constantly"; to: 0.6; info: root.vm.blitz; value: root.d.blitzFrequency || 0; onMoved: function(v) { root.update("defense", "blitzFrequency", v) } onReleased: Store.game.save() }
                    Tuner { label: "Run-stop commitment"; readout: ""; lo: "Deep safeties"; hi: "Loaded box"; value: root.d.runStopCommitment || 0; onMoved: function(v) { root.update("defense", "runStopCommitment", v) } onReleased: Store.game.save() }
                    Tuner { label: "Turnover hunting"; readout: ""; lo: "Gap discipline"; hi: "Go for the ball"; value: root.d.turnoverHunting || 0; onMoved: function(v) { root.update("defense", "turnoverHunting", v) } onReleased: Store.game.save() }
                }
            }

            // ---------------- special teams
            ColumnLayout {
                visible: root.tab === "Special teams"
                Layout.fillWidth: true; spacing: 12
                Card { title: "Kicking game"
                    Text { text: "Field goal range limit: " + Math.round(root.s.fieldGoalRangeLimit || 0) + " yards"; color: Theme.ink; font.bold: true }
                    Slide { Layout.fillWidth: true; from: 35; to: 65; stepSize: 1; value: root.s.fieldGoalRangeLimit || 52; onMoved: root.update("special_teams", "fieldGoalRangeLimit", value); onPressedChanged: if (!pressed) Store.game.save() }
                    Text { text: "Your kicker: " + (root.vm.kicker || ""); color: Theme.inkSubtle; font.pixelSize: 12 }
                    Text { text: "Onside kick tendency: " + Math.round((root.s.onsideTendency || 0) * 100) + "% when not desperate"; color: Theme.ink; font.bold: true; topPadding: 8 }
                    Slide { Layout.fillWidth: true; from: 0; to: 0.3; stepSize: 0.01; value: root.s.onsideTendency || 0; onMoved: root.update("special_teams", "onsideTendency", value); onPressedChanged: if (!pressed) Store.game.save() }
                    Text { text: "Trailing late, the team will try one anyway."; color: Theme.inkSubtle; font.pixelSize: 12 }
                }
            }
            Item { Layout.preferredHeight: 10 }
        }
    }
}
