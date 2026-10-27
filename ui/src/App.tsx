import { AppProvider, useApp } from "./lib/store";
import Sidebar from "./components/Sidebar";
import ChatScreen from "./screens/ChatScreen";
import HistoryScreen from "./screens/HistoryScreen";
import SkillsScreen from "./screens/SkillsScreen";
import RecipesScreen from "./screens/RecipesScreen";
import ExtensionsScreen from "./screens/ExtensionsScreen";
import SchedulerScreen from "./screens/SchedulerScreen";
import SettingsScreen from "./screens/SettingsScreen";

function Shell() {
  const { screen } = useApp();
  return (
    <div className="app-shell">
      <Sidebar />
      <main className="main">
        {screen === "chat" && <ChatScreen />}
        {screen === "history" && <HistoryScreen />}
        {screen === "skills" && <SkillsScreen />}
        {screen === "recipes" && <RecipesScreen />}
        {screen === "extensions" && <ExtensionsScreen />}
        {screen === "scheduler" && <SchedulerScreen />}
        {screen === "settings" && <SettingsScreen />}
      </main>
    </div>
  );
}

export default function App() {
  return (
    <AppProvider>
      <Shell />
    </AppProvider>
  );
}