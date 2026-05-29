import "./globals.css";
import type { Metadata } from "next";
import Link from "next/link";
import StatusPill from "./components/StatusPill";

export const metadata: Metadata = {
  title: "AgentSentry",
  description: "Runtime control plane for AI agents",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>
        <div className="app">
          <aside className="sidebar">
            <h1>AgentSentry</h1>
            <nav>
              <Link href="/">Dashboard</Link>
              <Link href="/traces">Traces</Link>
              <Link href="/agents">Agents</Link>
              <Link href="/policies">Policies</Link>
              <Link href="/onboarding">Onboarding</Link>
            </nav>
            <div className="sidebar-foot">
              <StatusPill />
            </div>
          </aside>
          <main className="main">{children}</main>
        </div>
      </body>
    </html>
  );
}
