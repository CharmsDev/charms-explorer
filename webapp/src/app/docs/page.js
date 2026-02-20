'use client';

import { useState } from 'react';

// ── Endpoint data ──────────────────────────────────────────────────────────
const SECTIONS = [
  {
    id: 'charms',
    title: 'Charms',
    badge: 'Explorer',
    badgeColor: 'blue',
    description: 'Browse and query on-chain charms. Used by the Explorer webapp.',
    endpoints: [
      {
        method: 'GET',
        path: '/v1/charms',
        desc: 'List all charms (paginated)',
        params: [
          { name: 'page', type: 'u64', required: false, desc: 'Page number (default: 1)' },
          { name: 'limit', type: 'u64', required: false, desc: 'Items per page (default: 20)' },
          { name: 'network', type: 'string', required: false, desc: 'mainnet | testnet4' },
        ],
        response: `{
  "data": { "charms": [...] },
  "pagination": { "total": 1234, "page": 1, "limit": 20, "total_pages": 62 }
}`,
      },
      {
        method: 'GET',
        path: '/v1/charms/{txid}',
        desc: 'Get charm by transaction ID',
        response: `{
  "txid": "abc...", "vout": 1,
  "app_id": "t/abc.../vk",
  "block_height": 210000,
  "asset_type": "token",
  "amount": 1000,
  "name": "BRO",
  "verified": true
}`,
      },
      {
        method: 'GET',
        path: '/v1/charms/by-charmid/{charmid}',
        desc: 'Get charm by app ID',
        response: '// Same shape as /v1/charms/{txid}',
      },
      {
        method: 'GET',
        path: '/v1/charms/by-address/{address}',
        desc: 'Unspent charms by Bitcoin address',
        response: `{
  "charms": [
    { "txid": "...", "vout": 1, "app_id": "t/...", "amount": 500, ... }
  ]
}`,
      },
      {
        method: 'GET',
        path: '/v1/charms/by-type',
        desc: 'Charms filtered by asset type',
        params: [
          { name: 'asset_type', type: 'string', required: true, desc: 'token, nft, or dapp' },
        ],
      },
      {
        method: 'GET',
        path: '/v1/charms/count-by-type',
        desc: 'Count charms grouped by asset type',
        response: '{ "total": 5000, "nft": 200, "token": 4700, "dapp": 100 }',
      },
      {
        method: 'POST',
        path: '/v1/charms/like',
        desc: 'Like / unlike a charm',
        body: '{ "charm_id": "t/abc...", "user_id": 1 }',
        note: 'Use POST to like, DELETE to unlike (same path).',
      },
    ],
  },
  {
    id: 'assets',
    title: 'Assets',
    badge: 'Explorer',
    badgeColor: 'blue',
    description: 'Unique asset registry (tokens, NFTs, dApps).',
    endpoints: [
      {
        method: 'GET',
        path: '/v1/assets',
        desc: 'List assets (paginated)',
        params: [
          { name: 'page', type: 'u64', required: false, desc: 'Page number' },
          { name: 'limit', type: 'u64', required: false, desc: 'Items per page' },
          { name: 'asset_type', type: 'string', required: false, desc: 'token, nft, dapp' },
          { name: 'network', type: 'string', required: false, desc: 'mainnet | testnet4' },
        ],
      },
      {
        method: 'GET',
        path: '/v1/assets/{asset_id}',
        desc: 'Asset details',
        response: `{
  "app_id": "t/abc.../vk",
  "asset_type": "token",
  "name": "BRO",
  "symbol": "BRO",
  "total_supply": 21000000
}`,
      },
      {
        method: 'GET',
        path: '/v1/assets/count',
        desc: 'Asset counts by type',
        response: '{ "total": 150, "nft": 50, "token": 90, "dapp": 10 }',
      },
      {
        method: 'GET',
        path: '/v1/assets/reference-nft/{hash}',
        desc: 'Reference NFT metadata for a token',
        response: `{
  "app_id": "n/abc.../0",
  "name": "BRO Token",
  "image_url": "https://...",
  "symbol": "BRO"
}`,
      },
      {
        method: 'GET',
        path: '/v1/assets/{app_id}/holders',
        desc: 'Top holders for an asset',
        response: `{
  "holders": [
    { "address": "bc1q...", "balance": 50000 }
  ],
  "total": 100
}`,
      },
    ],
  },
  {
    id: 'wallet',
    title: 'Wallet',
    badge: 'Wallet + Cast',
    badgeColor: 'green',
    description: 'BTC and Charm wallet operations. Used by Wallet Extension and Charms Cast. All endpoints accept ?network=mainnet|testnet4 (default: mainnet).',
    endpoints: [
      {
        method: 'GET',
        path: '/v1/wallet/utxos/{address}',
        desc: 'BTC UTXOs for an address',
        response: `{
  "address": "bc1q...",
  "utxos": [
    {
      "txid": "abc123...",
      "vout": 0,
      "value": 50000,
      "address": "bc1q...",
      "confirmations": 3,
      "confirmed": true
    }
  ],
  "count": 1
}`,
        note: 'Values in satoshis. Source: QuickNode (primary) → RPC (fallback).',
      },
      {
        method: 'GET',
        path: '/v1/wallet/balance/{address}',
        desc: 'BTC balance for an address',
        response: `{
  "address": "bc1q...",
  "confirmed": 150000,
  "unconfirmed": 5000,
  "total": 155000,
  "utxo_count": 3
}`,
        note: 'All values in satoshis.',
      },
      {
        method: 'GET',
        path: '/v1/wallet/charms/{address}',
        desc: 'Charm/token balances with per-UTXO details',
        response: `{
  "address": "bc1q...",
  "network": "mainnet",
  "balances": [
    {
      "appId": "t/abc123.../vk",
      "assetType": "token",
      "symbol": "BRO",
      "confirmed": 1000,
      "unconfirmed": 50,
      "total": 1050,
      "utxos": [
        {
          "txid": "def456...",
          "vout": 0,
          "value": 546,
          "address": "bc1q...",
          "appId": "t/abc123.../vk",
          "amount": 500,
          "confirmed": true,
          "blockHeight": 210000,
          "hasOrderCharm": false,
          "allCharmAppIds": ["t/abc123.../vk"]
        }
      ]
    }
  ],
  "count": 1
}`,
        note: 'Amounts are token units (not sats). hasOrderCharm: true if the UTXO also contains a DEX order charm (b/ prefix). allCharmAppIds: all charm app IDs on the same UTXO.',
      },
      {
        method: 'GET',
        path: '/v1/wallet/tx/{txid}',
        desc: 'Get raw transaction',
        response: '// Full transaction data from Bitcoin RPC (getrawtransaction verbose=true)',
      },
      {
        method: 'POST',
        path: '/v1/wallet/broadcast',
        desc: 'Broadcast a signed transaction',
        body: '{ "raw_tx": "0200000001..." }',
        response: '{ "txid": "abc123..." }',
      },
      {
        method: 'GET',
        path: '/v1/wallet/fee-estimate',
        desc: 'Fee rate estimate',
        params: [
          { name: 'blocks', type: 'u16', required: false, desc: 'Target blocks (default: 6)' },
        ],
        response: '{ "fee_rate": 0.00012, "blocks": 6 }',
        note: 'Fee rate in BTC/kB.',
      },
      {
        method: 'GET',
        path: '/v1/wallet/tip',
        desc: 'Current chain tip',
        response: '{ "height": 937396, "hash": "00000000...", "time": 1771501794 }',
      },
    ],
  },
  {
    id: 'dex',
    title: 'DEX Orders',
    badge: 'Cast + Explorer',
    badgeColor: 'purple',
    description: 'Order book for the Charms Cast DEX. Also displayed in the Explorer.',
    endpoints: [
      {
        method: 'GET',
        path: '/v1/dex/orders/open',
        desc: 'All open DEX orders',
        params: [
          { name: 'asset', type: 'string', required: false, desc: 'Filter by token app ID' },
          { name: 'side', type: 'string', required: false, desc: 'ask or bid' },
          { name: 'network', type: 'string', required: false, desc: 'mainnet | testnet4' },
        ],
        response: `{
  "total": 42,
  "orders": [
    {
      "order_id": "txid:vout",
      "txid": "abc123...",
      "vout": 0,
      "block_height": 210000,
      "platform": "scrolls",
      "maker": "bc1q...",
      "side": "ask",
      "exec_type": "limit",
      "price_num": 100,
      "price_den": 100000000,
      "price_per_token": 100.0,
      "amount": 50000,
      "quantity": 500,
      "filled_amount": 0,
      "filled_quantity": 0,
      "asset_app_id": "t/abc.../vk",
      "scrolls_address": "bc1q...",
      "status": "open",
      "confirmed": true,
      "parent_order_id": null,
      "created_at": "2025-01-15 10:30:00",
      "network": "mainnet"
    }
  ]
}`,
        note: 'amount: BTC in sats. quantity: token units. price_per_token: sats per token. confirmed: whether the order TX is confirmed on-chain.',
      },
      {
        method: 'GET',
        path: '/v1/dex/orders/{order_id}',
        desc: 'Single order by ID',
        response: '// Single order object (same shape as above) or null',
      },
      {
        method: 'GET',
        path: '/v1/dex/orders/by-asset/{asset_app_id}',
        desc: 'All orders for an asset (any status)',
        response: '// Same shape as /v1/dex/orders/open',
      },
      {
        method: 'GET',
        path: '/v1/dex/orders/by-maker/{maker}',
        desc: 'Orders by maker address',
        params: [
          { name: 'status', type: 'string', required: false, desc: 'open, filled, cancelled' },
        ],
        response: '// Same shape as /v1/dex/orders/open',
      },
    ],
  },
  {
    id: 'infra',
    title: 'Infrastructure',
    badge: 'Internal',
    badgeColor: 'orange',
    description: 'Health checks, status, and admin operations.',
    endpoints: [
      {
        method: 'GET',
        path: '/v1/health',
        desc: 'Health check',
        response: '// Returns 200 OK if the API is running',
      },
      {
        method: 'GET',
        path: '/v1/status',
        desc: 'Indexer status & stats',
        response: `{
  "networks": {
    "mainnet": {
      "indexer_status": { "status": "running", "last_processed_block": 937396, ... },
      "bitcoin_node": { "status": "online", "block_count": 937396 },
      "charm_stats": { "total_charms": 12345, "total_transactions": 5678 }
    }
  }
}`,
      },
      {
        method: 'GET',
        path: '/v1/diagnose',
        desc: 'Database diagnostics',
        response: '// Detailed database health and table stats',
      },
    ],
  },
];

// ── Badge colors ───────────────────────────────────────────────────────────
const BADGE_COLORS = {
  blue: 'bg-blue-500/15 text-blue-400',
  green: 'bg-green-500/15 text-green-400',
  purple: 'bg-purple-500/15 text-purple-400',
  orange: 'bg-orange-500/15 text-orange-400',
};

const METHOD_COLORS = {
  GET: 'bg-green-500/15 text-green-400',
  POST: 'bg-orange-500/15 text-orange-400',
  DELETE: 'bg-red-500/15 text-red-400',
};

// ── Components ─────────────────────────────────────────────────────────────

function EndpointCard({ ep }) {
  const [open, setOpen] = useState(false);
  const hasBody = ep.params || ep.body || ep.response || ep.note;

  return (
    <div className="border border-dark-700 rounded-lg overflow-hidden mb-3 bg-dark-800/50">
      <button
        onClick={() => hasBody && setOpen(!open)}
        className="w-full flex items-center gap-3 px-4 py-3 text-left hover:bg-dark-700/30 transition-colors"
      >
        <span className={`text-xs font-bold px-2 py-0.5 rounded ${METHOD_COLORS[ep.method] || 'bg-gray-500/15 text-gray-400'}`}>
          {ep.method}
        </span>
        <code className="text-sm text-white font-mono">{ep.path}</code>
        <span className="text-dark-400 text-xs ml-auto hidden sm:inline">{ep.desc}</span>
        {hasBody && (
          <span className={`text-dark-400 text-xs transition-transform ${open ? 'rotate-90' : ''}`}>▶</span>
        )}
      </button>

      {open && hasBody && (
        <div className="border-t border-dark-700 px-4 py-3 space-y-3">
          {/* Description on mobile */}
          <p className="text-dark-400 text-xs sm:hidden">{ep.desc}</p>

          {/* Params table */}
          {ep.params && (
            <div>
              <p className="text-[10px] font-semibold uppercase tracking-wider text-dark-400 mb-1">Parameters</p>
              <table className="w-full text-xs">
                <thead>
                  <tr className="text-dark-400 border-b border-dark-700">
                    <th className="text-left py-1 pr-3">Name</th>
                    <th className="text-left py-1 pr-3">Type</th>
                    <th className="text-left py-1 pr-3"></th>
                    <th className="text-left py-1">Description</th>
                  </tr>
                </thead>
                <tbody>
                  {ep.params.map((p) => (
                    <tr key={p.name} className="border-b border-dark-700/50">
                      <td className="py-1 pr-3"><code className="text-blue-400">{p.name}</code></td>
                      <td className="py-1 pr-3 text-purple-400 font-mono">{p.type}</td>
                      <td className="py-1 pr-3">
                        {p.required
                          ? <span className="text-red-400 text-[10px]">required</span>
                          : <span className="text-dark-500 text-[10px]">optional</span>
                        }
                      </td>
                      <td className="py-1 text-dark-300">{p.desc}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          {/* Request body */}
          {ep.body && (
            <div>
              <p className="text-[10px] font-semibold uppercase tracking-wider text-dark-400 mb-1">Request Body</p>
              <pre className="bg-dark-900 border border-dark-700 rounded p-3 text-xs overflow-x-auto font-mono text-dark-200">
                {ep.body}
              </pre>
            </div>
          )}

          {/* Response */}
          {ep.response && (
            <div>
              <p className="text-[10px] font-semibold uppercase tracking-wider text-dark-400 mb-1">Response</p>
              <pre className="bg-dark-900 border border-dark-700 rounded p-3 text-xs overflow-x-auto font-mono text-dark-200">
                {ep.response}
              </pre>
            </div>
          )}

          {/* Note */}
          {ep.note && (
            <div className={`text-xs px-3 py-2 rounded border-l-2 ${
              ep.noteColor === 'red'
                ? 'border-red-500 bg-red-500/5 text-red-300'
                : 'border-orange-500 bg-orange-500/5 text-dark-300'
            }`}>
              {ep.note}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ── Page ────────────────────────────────────────────────────────────────────

export default function DocsPage() {
  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl">
      {/* Header */}
      <h1 className="text-3xl font-bold mb-1">
        <span className="bg-gradient-to-r from-blue-400 to-blue-600 bg-clip-text text-transparent">Charms Explorer</span>
        {' '}API{' '}
        <code className="text-lg text-blue-400">v1</code>
      </h1>
      <p className="text-dark-400 text-sm mb-6">
        Unified API — Base URL: <code className="text-blue-400">{'{host}'}/v1</code> — Used by Explorer, Wallet Extension &amp; Charms Cast
      </p>

      {/* Versioning note */}
      <div className="border-l-2 border-orange-500 bg-orange-500/5 px-4 py-3 rounded-r text-sm text-dark-300 mb-8">
        <strong className="text-orange-400">Versioning:</strong> All endpoints are served under <code className="text-blue-400">/v1/</code> (canonical).
        Legacy routes without prefix still work for backward compatibility but new integrations should use <code className="text-blue-400">/v1/</code>.
      </div>

      {/* Nav */}
      <nav className="flex flex-wrap gap-2 mb-8">
        {SECTIONS.map((s) => (
          <a
            key={s.id}
            href={`#${s.id}`}
            className={`text-xs font-medium px-3 py-1.5 rounded-md border border-dark-700 hover:bg-dark-700/50 transition-colors ${
              BADGE_COLORS[s.badgeColor]?.split(' ')[1] || 'text-dark-300'
            }`}
          >
            {s.title}
          </a>
        ))}
      </nav>

      {/* Sections */}
      {SECTIONS.map((section) => (
        <section key={section.id} id={section.id} className="mb-12">
          <div className="flex items-center gap-3 mb-1">
            <h2 className="text-xl font-bold text-white">{section.title}</h2>
            <span className={`text-[10px] font-semibold uppercase px-2 py-0.5 rounded-full ${BADGE_COLORS[section.badgeColor]}`}>
              {section.badge}
            </span>
          </div>
          <p className="text-dark-400 text-sm mb-4">{section.description}</p>

          {section.endpoints.map((ep, i) => (
            <EndpointCard key={i} ep={ep} />
          ))}
        </section>
      ))}

      {/* Footer */}
      <div className="text-center text-dark-500 text-xs pt-8 border-t border-dark-700">
        Charms Explorer API v1 — Internal Documentation
      </div>
    </div>
  );
}
