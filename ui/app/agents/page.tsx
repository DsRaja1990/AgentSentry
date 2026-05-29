import { api, type Agent } from "@/lib/api";

export const dynamic = "force-dynamic";

export default async function Agents() {
  let items: Agent[] = [];
  let error: string | null = null;
  try { items = (await api.agents()).items ?? []; }
  catch (e: unknown) { error = e instanceof Error ? e.message : String(e); }

  return (
    <>
      <h2>Agents</h2>
      {error && <div className="error">{error}</div>}
      {items.length === 0 && !error && <p style={{color:"var(--muted)"}}>No agents registered.</p>}
      {items.length > 0 && (
        <table>
          <thead><tr><th>Name</th><th>ID</th><th>Framework</th><th>Owner</th><th>Created</th></tr></thead>
          <tbody>
            {items.map(a => (
              <tr key={a.id}>
                <td>{a.name}</td>
                <td><code>{a.id}</code></td>
                <td>{a.framework}</td>
                <td>{a.owner}</td>
                <td>{new Date(a.created_at).toLocaleString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </>
  );
}
