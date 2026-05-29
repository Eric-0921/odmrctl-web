import TopStatusBar from "./TopStatusBar";
import SideNav from "./SideNav";
import MockOnlyBanner from "./MockOnlyBanner";
import { MockModeProvider } from "../context/MockModeContext";

interface AppShellProps {
  children: React.ReactNode;
}

export default function AppShell({ children }: AppShellProps) {
  return (
    <MockModeProvider>
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          height: "100vh",
          overflow: "hidden",
        }}
      >
        <TopStatusBar />
        <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
          <SideNav />
          <main
            style={{
              flex: 1,
              display: "flex",
              flexDirection: "column",
              overflow: "hidden",
            }}
          >
            <MockOnlyBanner />
            <div
              style={{
                flex: 1,
                overflow: "auto",
                padding: "var(--space-6)",
              }}
            >
              {children}
            </div>
          </main>
        </div>
      </div>
    </MockModeProvider>
  );
}
