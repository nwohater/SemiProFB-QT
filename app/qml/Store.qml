pragma Singleton
import QtQuick
import com.semipro

QtObject {
    readonly property Game game: Game {}

    // Jersey art on disk: team slug -> file name.
    readonly property var jerseyArt: JSON.parse(game.jerseysJson())

    // Screens ask the shell to open dialogs.
    signal playerRequested(int id)
    signal financeTabRequested(string tab)
    signal negotiateRequested(int id)
    signal storyRequested(string kind)
    function openStory(k) { storyRequested(k); }
    signal boxRequested(int index)
    function openBox(i) { boxRequested(i); }
    function openPlayer(id) { playerRequested(id); }
    function openFinanceTab(tab) { financeTabRequested(tab); }
    function openNegotiation(id) { negotiateRequested(id); }
}
