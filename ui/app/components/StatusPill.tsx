"use client";
import { useEffect, useState } from "react";

export default function StatusPill() {
  const [state, setState] = useState<"checking"|"ok"|"down">("checking");
  useEffect(() => {
    let alive = true;
    const tick = async () => {
      try {
        const r = await fetch("/api/proxy/v1/health", { cache: "no-store" });
        if (alive) setState(r.ok ? "ok" : "down");
      } catch { if (alive) setState("down"); }
    };
    tick();
    const id = setInterval(tick, 10_000);
    return () => { alive = false; clearInterval(id); };
  }, []);
  const cls = state === "ok" ? "pill pill-ok" : state === "down" ? "pill pill-down" : "pill pill-muted";
  const label = state === "ok" ? "control plane online" : state === "down" ? "control plane unreachable" : "checking…";
  return <span className={cls} title={label}><span className="pill-dot" />{label}</span>;
}
