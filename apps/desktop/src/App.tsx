import { Routes, Route } from "react-router-dom";
import AppShell from "./components/AppShell";
import DashboardPage from "./routes/DashboardPage";
import DevicesPage from "./routes/DevicesPage";
import RecipePage from "./routes/RecipePage";
import DryRunPage from "./routes/DryRunPage";
import SafetyPage from "./routes/SafetyPage";
import EventsPage from "./routes/EventsPage";
import RawDataPreviewPage from "./routes/RawDataPreviewPage";
import AboutBoundariesPage from "./routes/AboutBoundariesPage";

function App() {
  return (
    <AppShell>
      <Routes>
        <Route path="/" element={<DashboardPage />} />
        <Route path="/devices" element={<DevicesPage />} />
        <Route path="/recipe" element={<RecipePage />} />
        <Route path="/dry-run" element={<DryRunPage />} />
        <Route path="/safety" element={<SafetyPage />} />
        <Route path="/events" element={<EventsPage />} />
        <Route path="/raw-data" element={<RawDataPreviewPage />} />
        <Route path="/about" element={<AboutBoundariesPage />} />
      </Routes>
    </AppShell>
  );
}

export default App;
