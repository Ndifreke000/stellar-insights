import { useCallback, useMemo, useState } from 'react';

type PreflightResponse = {
  source_asset?: string;
  destination_asset?: string;
  amount_usd?: number;
  [key: string]: unknown;
};

type RequestState =
  | { status: 'idle' }
  | { status: 'loading' }
  | { status: 'error'; message: string }
  | { status: 'success'; data: PreflightResponse };

export type PreflightInputs = {
  sourceAsset: string;
  destinationAsset: string;
  amountUsd: string;
};

const API_BASE = import.meta.env?.VITE_API_BASE_URL ?? 'http://localhost:8080';

function buildQuery(inputs: PreflightInputs): string {
  const params = new URLSearchParams();
  if (inputs.sourceAsset.trim()) params.set('source_asset', inputs.sourceAsset.trim());
  if (inputs.destinationAsset.trim()) params.set('destination_asset', inputs.destinationAsset.trim());
  if (inputs.amountUsd.trim()) params.set('amount_usd', inputs.amountUsd.trim());
  return params.toString();
}

export function buildCurlSnippet(inputs: PreflightInputs, baseUrl: string = API_BASE): string {
  const query = buildQuery(inputs);
  return `curl -s "${baseUrl}/api/v1/preflight?${query}"`;
}

export function buildTypeScriptSnippet(inputs: PreflightInputs, baseUrl: string = API_BASE): string {
  const query = buildQuery(inputs);
  return [
    `const res = await fetch(`,
    `  "${baseUrl}/api/v1/preflight?${query}",`,
    `);`,
    `if (!res.ok) throw new Error(\`Preflight failed: \${res.status}\`);`,
    `const preflight = await res.json();`,
    `console.log(preflight);`,
  ].join('\n');
}

export function buildPythonSnippet(inputs: PreflightInputs, baseUrl: string = API_BASE): string {
  const query = buildQuery(inputs);
  return [
    `import requests`,
    ``,
    `res = requests.get(`,
    `    "${baseUrl}/api/v1/preflight?${query}",`,
    `    timeout=10,`,
    `)`,
    `res.raise_for_status()`,
    `preflight = res.json()`,
    `print(preflight)`,
  ].join('\n');
}

function SnippetBlock({ label, code }: { label: string; code: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      setCopied(false);
    }
  }, [code]);

  return (
    <div className="snippet-block">
      <div className="snippet-header">
        <span>{label}</span>
        <button type="button" onClick={handleCopy}>
          {copied ? 'Copied' : 'Copy'}
        </button>
      </div>
      <pre>
        <code>{code}</code>
      </pre>
    </div>
  );
}

export default function PreflightPlayground() {
  const [inputs, setInputs] = useState<PreflightInputs>({
    sourceAsset: 'USDC',
    destinationAsset: 'NGN',
    amountUsd: '2500',
  });
  const [state, setState] = useState<RequestState>({ status: 'idle' });

  const snippets = useMemo(
    () => ({
      curl: buildCurlSnippet(inputs),
      typescript: buildTypeScriptSnippet(inputs),
      python: buildPythonSnippet(inputs),
    }),
    [inputs],
  );

  const update = (key: keyof PreflightInputs) => (event: React.ChangeEvent<HTMLInputElement>) => {
    setInputs((prev) => ({ ...prev, [key]: event.target.value }));
  };

  const runPreflight = useCallback(async () => {
    const query = buildQuery(inputs);
    if (!query) {
      setState({ status: 'error', message: 'Provide at least one input to run a preflight.' });
      return;
    }
    setState({ status: 'loading' });
    try {
      const res = await fetch(`${API_BASE}/api/v1/preflight?${query}`);
      if (!res.ok) {
        const text = await res.text().catch(() => '');
        setState({
          status: 'error',
          message: `Request failed (${res.status})${text ? `: ${text}` : ''}`,
        });
        return;
      }
      const data = (await res.json()) as PreflightResponse;
      setState({ status: 'success', data });
    } catch (err) {
      setState({
        status: 'error',
        message: err instanceof Error ? err.message : 'Network error',
      });
    }
  }, [inputs]);

  return (
    <div className="preflight-playground">
      <h1>Preflight Playground</h1>
      <p>Try the preflight endpoint with live inputs and copy a ready-to-run snippet.</p>

      <form
        className="preflight-form"
        onSubmit={(event) => {
          event.preventDefault();
          void runPreflight();
        }}
      >
        <label>
          Source asset
          <input value={inputs.sourceAsset} onChange={update('sourceAsset')} placeholder="USDC" />
        </label>
        <label>
          Destination asset
          <input
            value={inputs.destinationAsset}
            onChange={update('destinationAsset')}
            placeholder="NGN"
          />
        </label>
        <label>
          Amount (USD)
          <input
            value={inputs.amountUsd}
            onChange={update('amountUsd')}
            placeholder="2500"
            inputMode="decimal"
          />
        </label>
        <button type="submit" disabled={state.status === 'loading'}>
          {state.status === 'loading' ? 'Running…' : 'Run preflight'}
        </button>
      </form>

      <section className="preflight-response" aria-live="polite">
        <h2>Response</h2>
        {state.status === 'idle' && <p>Run a preflight to see the response.</p>}
        {state.status === 'loading' && <p>Loading…</p>}
        {state.status === 'error' && (
          <p role="alert" className="error">
            {state.message}
          </p>
        )}
        {state.status === 'success' && (
          <>
            {Object.keys(state.data).length === 0 ? (
              <p>No data returned for these inputs.</p>
            ) : (
              <pre>
                <code>{JSON.stringify(state.data, null, 2)}</code>
              </pre>
            )}
          </>
        )}
      </section>

      <section className="preflight-snippets">
        <h2>Code snippets</h2>
        <SnippetBlock label="curl" code={snippets.curl} />
        <SnippetBlock label="TypeScript" code={snippets.typescript} />
        <SnippetBlock label="Python" code={snippets.python} />
      </section>
    </div>
  );
}
