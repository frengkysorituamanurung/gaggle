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

function Shell() {
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
          <Route path="/extensions" element={<ExtensionsScreen />} />
          <Route path="/scheduler" element={<SchedulerScreen />} />
          <Route path="/settings" element={<SettingsScreen />} />
          <Route path="/memories" element={<MemoriesScreen />} />
          <Route path="/permissions" element={<PermissionScreen />} />
          <Route path="/workspace" element={<WorkspaceScreen />} />
          <Route path="/terminal" element={<TerminalScreen />} />
          <Route path="/standalone-app" element={<StandaloneAppView />} />
          <Route path="/launcher" element={<LauncherView />} />
          <Route path="/configure-providers" element={<ConfigureProvidersScreen />} />
        </Routes>
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