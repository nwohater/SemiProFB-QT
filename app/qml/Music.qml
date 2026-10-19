pragma Singleton
import QtQuick
import QtMultimedia

// Shuffled background music: plays tracks at random (never the same one twice in a row) and moves on when one ends.
// Settings (on/off, volume) live in settings.json next to the save.
QtObject {
    id: music
    property bool enabled: true
    property real volume: 0.5
    property var tracks: []
    property string nowPlaying: ""        // file name
    property bool unavailable: false
    property bool started: false
    readonly property string nowTitle: nowPlaying.replace(/\.mp3$/i, "")

    readonly property AudioOutput out: AudioOutput { volume: music.volume }
    readonly property MediaPlayer player: MediaPlayer {
        audioOutput: music.out
        onMediaStatusChanged: if (mediaStatus === MediaPlayer.EndOfMedia) music.next()
        onErrorOccurred: function(error, errorString) { music.unavailable = true; console.log("music:", errorString); }
    }

    function start() {
        if (started) return;
        started = true;
        try {
            var s = JSON.parse(Store.game.settingsJson());
            if (s.musicOn !== undefined) enabled = s.musicOn;
            if (s.musicVolume !== undefined) volume = Math.max(0, Math.min(1, s.musicVolume));
            tracks = JSON.parse(Store.game.musicJson());
        } catch (e) { tracks = []; }
        if (enabled) next();
    }
    function next() {
        if (tracks.length === 0) return;
        var pool = tracks.filter(function(t) { return t !== nowPlaying; });
        if (pool.length === 0) pool = tracks;
        nowPlaying = pool[Math.floor(Math.random() * pool.length)];
        player.source = Store.game.assetsUrl + "music/" + encodeURIComponent(nowPlaying);
        if (enabled) player.play();
    }
    function setEnabled(on) {
        enabled = on;
        Store.game.setSetting("musicOn", JSON.stringify(on));
        if (on) { if (nowPlaying === "") next(); else player.play(); } else player.pause();
    }
    function setVolume(v) { volume = v; }
    function saveVolume() { Store.game.setSetting("musicVolume", JSON.stringify(volume)); }
}
