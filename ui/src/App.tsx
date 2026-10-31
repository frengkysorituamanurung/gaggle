import { BrowserRouter, Routes, Route } from "react-router-dom";
import { ToastContainer } from "react-toastify";
import "react-toastify/dist/ReactToastify.css";
import { AppProvider } from "./lib/store";
import { NavigationProvider } from "./components/Layout/NavigationContext";
import { AppLayout } from "./components/Layout/AppLayout";
import { Navigation } from "./components/Layout/NavigationPanel";
import { defineMessages, useIntl } from "./i18n";
import ChatScreen from "./screens/ChatScreen";
import HubScreen from "./screens/HubScreen";
import DiagnosticsScreen from "./screens/DiagnosticsScreen";
import WorktreeScreen from "./screens/WorktreeScreen";
import ContextScreen from "./screens/ContextScreen";
import MemoryManagementScreen from "./screens/MemoryManagementScreen";
import RecipeManagementScreen from "./screens/RecipeManagementScreen";
import ExtensionManagementScreen from "./screens/ExtensionManagementScreen";
import QuickLauncher from "./components/QuickLauncher";
import OnboardingGuard from "./components/onboarding/OnboardingGuard";
import AnnouncementModal from "./components/AnnouncementModal";
import TelemetryConsentPrompt from "./components/TelemetryConsentPrompt";
import HistoryScreen from "./screens/HistoryScreen";
import SkillsScreen from "./screens/SkillsScreen";
import RecipesScreen from "./screens/RecipesScreen";
import ExtensionsScreen from "./screens/ExtensionsScreen";
import SchedulerScreen from "./screens/SchedulerScreen";
import SettingsScreen from "./screens/SettingsScreen";
import MemoriesScreen from "./screens/MemoriesScreen";
import PermissionScreen from "./screens/PermissionScreen";
import WorkspaceScreen from "./screens/WorkspaceScreen";
import TerminalScreen from "./screens/TerminalScreen";
import PairScreen from "./screens/PairScreen";
import ConfigureProvidersScreen from "./screens/ConfigureProvidersScreen";
import LauncherView from "./components/LauncherView";
import AppsView from "./components/apps/AppsView";
import StandaloneAppView from "./components/apps/StandaloneAppView";

import React from "react";

function Shell() {
  const [launcherOpen, setLauncherOpen] = React.useState(false);

  React.useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setLauncherOpen((prev) => !prev);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  return (
    <NavigationProvider>
      <OnboardingGuard>
        <AppLayout sidebar={<Navigation />}>
          <Routes>
          <Route path="/" element={<HubScreen />} />
          <Route path="/chat" element={<ChatScreen />} />
          <Route path="/pair" element={<PairScreen />} />
          <Route path="/apps" element={<AppsView />} />
          <Route path="/history" element={<HistoryScreen />} />
          <Route path="/skills" element={<SkillsScreen />} />
          <Route path="/recipes" element={<RecipesScreen />} />
          <Route path="/recipes/manage" element={<RecipeManagementScreen />} />
          <Route path="/extensions" element={<ExtensionsScreen />} />
          <Route path="/extensions/manage" element={<ExtensionManagementScreen />} />
          <Route path="/scheduler" element={<SchedulerScreen />} />
          <Route path="/settings" element={<SettingsScreen />} />
          <Route path="/memories" element={<MemoriesScreen />} />
          <Route path="/memories/manage" element={<MemoryManagementScreen />} />
          <Route path="/permissions" element={<PermissionScreen />} />
          <Route path="/workspace" element={<WorkspaceScreen />} />
          <Route path="/terminal" element={<TerminalScreen />} />
          <Route path="/diagnostics" element={<DiagnosticsScreen />} />
          <Route path="/worktrees" element={<WorktreeScreen projectId="" />} />
          <Route path="/context" element={<ContextScreen chatId="" />} />
          <Route path="/standalone-app" element={<StandaloneAppView />} />
          <Route path="/launcher" element={<LauncherView />} />
          <Route path="/configure-providers" element={<ConfigureProvidersScreen />} />
        </Routes>
        <QuickLauncher open={launcherOpen} onClose={() => setLauncherOpen(false)} />
        </AppLayout>
      </OnboardingGuard>
    </NavigationProvider>
  );
}

const appMessages = defineMessages({
  toastNotifications: { id: "app.toastNotifications", defaultMessage: "Toast notifications" },
});

export default function App() {
  const intl = useIntl();
  return (
    <AppProvider>
      <BrowserRouter>
        <div className="titlebar-drag-region" />
        <Shell />
        <AnnouncementModal />
        <TelemetryConsentPrompt />
        <ToastContainer
          aria-label={intl.formatMessage(appMessages.toastNotifications)}
          position="top-right"
          autoClose={3000}
          closeOnClick
          pauseOnHover
          toastClassName={() =>
            "relative min-h-16 mb-4 p-2 rounded-lg flex justify-between overflow-hidden cursor-pointer text-text-inverse bg-background-inverse"
          }
          style={{ width: "450px" }}
          className="mt-6"
        />
      </BrowserRouter>
    </AppProvider>
  );
}