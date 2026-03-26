import { useState, useEffect, useCallback } from 'react'

interface NodeStatus {
  node_id: string
  version: string
  status: string
  uptime_secs: number
  data_count: number
}

interface NetworkHealth {
  status: string
  connected_peers: number
  network_score: number
}

interface DataItem {
  id: string
  domain: string
  score: number
}

interface Peer {
  node_id: string
  address: string
}

const API_BASE = '/api/v1'

function App() {
  const [nodeStatus, setNodeStatus] = useState<NodeStatus | null>(null)
  const [networkHealth, setNetworkHealth] = useState<NetworkHealth | null>(null)
  const [data, setData] = useState<DataItem[]>([])
  const [peers, setPeers] = useState<Peer[]>([])
  const [error, setError] = useState<string | null>(null)
  const [insertContent, setInsertContent] = useState('')
  const [insertDomain, setInsertDomain] = useState('climate')
  const [insertResult, setInsertResult] = useState<string | null>(null)

  const fetchAll = useCallback(async () => {
    try {
      const [statusRes, healthRes, dataRes, peersRes] = await Promise.all([
        fetch(`${API_BASE}/node/status`),
        fetch(`${API_BASE}/network/health`),
        fetch(`${API_BASE}/data/query`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ query: '', limit: 50 }),
        }),
        fetch(`${API_BASE}/node/peers`),
      ])
      if (statusRes.ok) setNodeStatus(await statusRes.json())
      if (healthRes.ok) setNetworkHealth(await healthRes.json())
      if (dataRes.ok) setData(await dataRes.json())
      if (peersRes.ok) setPeers(await peersRes.json())
      setError(null)
    } catch (e) {
      setError(`Connessione al nodo fallita: ${e}`)
    }
  }, [])

  useEffect(() => {
    fetchAll()
    const interval = setInterval(fetchAll, 3000)
    return () => clearInterval(interval)
  }, [fetchAll])

  const handleInsert = async () => {
    if (!insertContent.trim()) return
    try {
      const res = await fetch(`${API_BASE}/data`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          content: insertContent,
          domain: insertDomain,
          source: 'dashboard',
        }),
      })
      const json = await res.json()
      setInsertResult(
        res.ok ? `Inserito: ${json.id?.substring(0, 16)}... (score: ${json.score?.toFixed(2)})` : `Errore: ${json.error}`
      )
      setInsertContent('')
      fetchAll()
    } catch (e) {
      setInsertResult(`Errore: ${e}`)
    }
  }

  const formatUptime = (secs: number) => {
    const h = Math.floor(secs / 3600)
    const m = Math.floor((secs % 3600) / 60)
    const s = secs % 60
    return h > 0 ? `${h}h ${m}m ${s}s` : m > 0 ? `${m}m ${s}s` : `${s}s`
  }

  return (
    <div style={{ maxWidth: 960, margin: '0 auto', padding: 24 }}>
      <header style={{ borderBottom: '1px solid #333', paddingBottom: 16, marginBottom: 24 }}>
        <h1 style={{ fontSize: 28, color: '#7cb3ff' }}>VARCAVIA Dashboard</h1>
        <p style={{ color: '#888', fontSize: 14 }}>Decentralized Clean Data Infrastructure</p>
      </header>

      {error && (
        <div style={{ background: '#3a1111', border: '1px solid #ff4444', borderRadius: 8, padding: 12, marginBottom: 16 }}>
          {error}
        </div>
      )}

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 16, marginBottom: 24 }}>
        <Card title="Nodo">
          {nodeStatus ? (
            <>
              <Stat label="Status" value={nodeStatus.status} color="#4caf50" />
              <Stat label="Uptime" value={formatUptime(nodeStatus.uptime_secs)} />
              <Stat label="Dati" value={String(nodeStatus.data_count)} color="#7cb3ff" />
              <Stat label="ID" value={nodeStatus.node_id.substring(0, 16) + '...'} />
            </>
          ) : <p style={{ color: '#666' }}>Caricamento...</p>}
        </Card>

        <Card title="Rete">
          {networkHealth ? (
            <>
              <Stat label="Status" value={networkHealth.status} color={networkHealth.status === 'healthy' ? '#4caf50' : '#ff9800'} />
              <Stat label="Peer connessi" value={String(networkHealth.connected_peers)} color="#7cb3ff" />
              <Stat label="Score rete" value={networkHealth.network_score.toFixed(2)} />
            </>
          ) : <p style={{ color: '#666' }}>Caricamento...</p>}
        </Card>

        <Card title="Peer">
          {peers.length > 0 ? (
            peers.map((p, i) => (
              <div key={i} style={{ fontSize: 13, padding: '4px 0', borderBottom: '1px solid #222' }}>
                <span style={{ color: '#4caf50' }}>{p.address}</span>
              </div>
            ))
          ) : <p style={{ color: '#666' }}>Nessun peer connesso</p>}
        </Card>
      </div>

      <Card title="Inserisci Dato">
        <div style={{ display: 'flex', gap: 8, marginBottom: 8 }}>
          <input
            value={insertContent}
            onChange={e => setInsertContent(e.target.value)}
            placeholder="Contenuto del dato..."
            style={{ flex: 1, padding: 8, background: '#1a1a2e', border: '1px solid #333', borderRadius: 4, color: '#e0e0e0' }}
            onKeyDown={e => e.key === 'Enter' && handleInsert()}
          />
          <select
            value={insertDomain}
            onChange={e => setInsertDomain(e.target.value)}
            style={{ padding: 8, background: '#1a1a2e', border: '1px solid #333', borderRadius: 4, color: '#e0e0e0' }}
          >
            <option value="climate">Climate</option>
            <option value="health">Health</option>
            <option value="finance">Finance</option>
            <option value="science">Science</option>
            <option value="general">General</option>
          </select>
          <button
            onClick={handleInsert}
            style={{ padding: '8px 20px', background: '#1a5fb4', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer' }}
          >
            Inserisci
          </button>
        </div>
        {insertResult && <p style={{ fontSize: 13, color: insertResult.startsWith('Errore') ? '#ff4444' : '#4caf50' }}>{insertResult}</p>}
      </Card>

      <div style={{ marginTop: 24 }}>
        <Card title={`Dati (${data.length})`}>
          {data.length > 0 ? (
            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
              <thead>
                <tr style={{ borderBottom: '1px solid #333' }}>
                  <th style={{ textAlign: 'left', padding: 8, color: '#888' }}>ID</th>
                  <th style={{ textAlign: 'left', padding: 8, color: '#888' }}>Dominio</th>
                  <th style={{ textAlign: 'right', padding: 8, color: '#888' }}>Score</th>
                </tr>
              </thead>
              <tbody>
                {data.map((item, i) => (
                  <tr key={i} style={{ borderBottom: '1px solid #1a1a2e' }}>
                    <td style={{ padding: 8, fontFamily: 'monospace' }}>{item.id.substring(0, 20)}...</td>
                    <td style={{ padding: 8 }}>
                      <span style={{ background: '#1a1a2e', padding: '2px 8px', borderRadius: 12, fontSize: 12 }}>{item.domain}</span>
                    </td>
                    <td style={{ padding: 8, textAlign: 'right' }}>
                      <span style={{ color: item.score > 0.7 ? '#4caf50' : item.score > 0.4 ? '#ff9800' : '#ff4444' }}>
                        {item.score.toFixed(2)}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : <p style={{ color: '#666' }}>Nessun dato inserito</p>}
        </Card>
      </div>
    </div>
  )
}

function Card({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={{ background: '#111122', borderRadius: 8, padding: 16, border: '1px solid #222' }}>
      <h3 style={{ fontSize: 14, color: '#888', marginBottom: 12, textTransform: 'uppercase', letterSpacing: 1 }}>{title}</h3>
      {children}
    </div>
  )
}

function Stat({ label, value, color }: { label: string; value: string; color?: string }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', padding: '4px 0' }}>
      <span style={{ color: '#888', fontSize: 13 }}>{label}</span>
      <span style={{ color: color || '#e0e0e0', fontSize: 13, fontFamily: 'monospace' }}>{value}</span>
    </div>
  )
}

export default App
