import { api } from "@/lib/api";

export const dynamic = "force-dynamic";

export default async function PolicyDetail({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  try {
    const p = await api.policy(id);
    return (
      <>
        <h2>{p.name}</h2>
        <p style={{color:"var(--muted)"}}>
          <code>{p.id}</code> · {p.language} · v{p.version} ·{" "}
          <span className={`badge ${p.status === "enforced" ? "badge-deny" : "badge-default"}`}>{p.status}</span>
        </p>
        <pre>{p.source}</pre>
      </>
    );
  } catch (e: unknown) {
    return <div className="error">{e instanceof Error ? e.message : String(e)}</div>;
  }
}
