import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { AppProvider } from "./lib/store";
import Sidebar from "./components/Sidebar";
import ChatScreen from "./screens/ChatScreen";
import HistoryScreen from "./screens/HistoryScreen";
import SkillsScreen from "./screens/SkillsScreen";
import RecipesScreen from "./screens/RecipesScreen";
import ExtensionsScreen from "./screens/ExtensionsScreen";
import SchedulerScreen from "./screens/SchedulerScreen";
import SettingsScreen from "./screens/SettingsScreen";

function Shell() {
  return (
    <div className="flex h-screen w-screen gap-2 bg-background-primary p-2 overflow-hidden">
      <Sidebar />
      <main className="flex-1 min-w-0 overflow-hidden rounded-xl pt-8">
        <Routes>
          <Route path="/" element={<Navigate to="/chat" replace />} />
          <Route path="/chat" element={<ChatScreen />} />
          <Route path="/history" element={<HistoryScreen />} />
          <Route path="/skills" element={<SkillsScreen />} />
          <Route path="/recipes" element={<RecipesScreen />} />
          <Route path="/extensions" element={<ExtensionsScreen />} />
          <Route path="/scheduler" element={<SchedulerScreen />} />
          <Route path="/settings" element={<SettingsScreen />} />
        </Routes>
      </main>
    </div>
  );
}

export default function App() {
  return (
    <AppProvider>
      <BrowserRouter>
        <Shell />
      </BrowserRouter>
    </AppProvider>
  );
}