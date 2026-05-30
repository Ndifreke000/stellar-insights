'use client';

import React, { useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { 
  Copy, 
  CheckCircle, 
  Code2, 
  Terminal,
  Globe,
  Database,
  Network,
  Zap
} from 'lucide-react';

interface CodeExample {
  title: string;
  description: string;
  language: string;
  code: string;
  endpoint?: string;
}

const codeExamples: Record<string, CodeExample[]> = {
  corridors: [
    {
      title: "List All Corridors",
      description: "Fetch all payment corridors sorted by success rate",
      language: "javascript",
      code: `// Fetch top corridors by success rate
async function fetchTopCorridors() {
  try {
    const response = await fetch(
      'http://localhost:8080/api/corridors?sort_by=success_rate&limit=10'
    );
    
    if (!response.ok) {
      throw new Error('HTTP error! status: ' + response.status);
    }
    
    const data = await response.json();
    console.log('Top corridors:', data.corridors);
    
    return data.corridors;
  } catch (error) {
    console.error('Error fetching corridors:', error);
  }
}

// Usage
fetchTopCorridors();`,
      endpoint: "GET /api/corridors"
    },
    {
      title: "Get Specific Corridor",
      description: "Retrieve detailed metrics for a specific asset pair",
      language: "python",
      code: `import requests
import json

def get_corridor_details():
    """Get detailed metrics for USDC-EURC corridor"""
    
    asset_pair = "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN->EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2"
    url = f"http://localhost:8080/api/corridors/{asset_pair}"
    
    try:
        response = requests.get(url)
        response.raise_for_status()
        
        corridor_data = response.json()
        print(f"Success Rate: {corridor_data['success_rate']}%")
        print(f"Volume: ${corridor_data['volume_usd']:,.2f}")
        print(f"Total Transactions: {corridor_data['total_transactions']}")
        
        return corridor_data
        
    except requests.exceptions.RequestException as e:
        print(f"Error: {e}")
        return None

# Usage
get_corridor_details()`,
      endpoint: "GET /api/corridors/{asset_pair}"
    },
    {
      title: "Filter Corridors by Volume",
      description: "Get high-volume corridors above a threshold",
      language: "javascript",
      code: `// Get corridors with volume > $1M
async function getHighVolumeCorridors() {
  const response = await fetch(
    'http://localhost:8080/api/corridors?sort_by=volume&limit=50'
  );
  
  const data = await response.json();
  
  // Filter corridors with volume > $1M
  const highVolumeCorridors = data.corridors.filter(
    corridor => corridor.volume_usd > 1000000
  );
  
  console.log('Found ' + highVolumeCorridors.length + ' high-volume corridors');
  return highVolumeCorridors;
}

// Usage
getHighVolumeCorridors();`,
      endpoint: "GET /api/corridors?sort_by=volume"
    }
  ],
  anchors: [
    {
      title: "List All Anchors",
      description: "Get all anchors with their reliability scores",
      language: "python",
      code: `import requests

def get_anchors_performance():
    """Fetch all anchors with their metrics"""
    
    url = "http://localhost:8080/api/anchors"
    
    try:
        response = requests.get(url)
        response.raise_for_status()
        
        anchors = response.json()
        
        # Sort by reliability score
        sorted_anchors = sorted(
            anchors, 
            key=lambda x: x.get('reliability_score', 0), 
            reverse=True
        )
        
        for anchor in sorted_anchors[:5]:  # Top 5
            print(f"Anchor: {anchor['name']}")
            print(f"Reliability: {anchor['reliability_score']}%")
            print(f"Assets: {len(anchor.get('assets', []))}")
            print("---")
        
        return sorted_anchors
        
    except requests.exceptions.RequestException as e:
        print(f"Error: {e}")
        return None

# Usage
get_anchors_performance()`,
      endpoint: "GET /api/anchors"
    },
    {
      title: "Get Anchor Assets",
      description: "Retrieve all assets issued by a specific anchor",
      language: "javascript",
      code: `// Get all assets for a specific anchor
async function getAnchorAssets(anchorId) {
  try {
    const response = await fetch(
      'http://localhost:8080/api/anchors/' + anchorId + '/assets'
    );
    
    if (!response.ok) {
      throw new Error('HTTP error! status: ' + response.status);
    }
    
    const assets = await response.json();
    
    // Group assets by type
    const assetTypes = assets.reduce((acc, asset) => {
      const type = asset.asset_code === 'native' ? 'XLM' : 'Stablecoin';
      acc[type] = (acc[type] || 0) + 1;
      return acc;
    }, {});
    
    console.log('Asset distribution:', assetTypes);
    return assets;
    
  } catch (error) {
    console.error('Error fetching anchor assets:', error);
  }
}

// Usage
getAnchorAssets(1);`,
      endpoint: "GET /api/anchors/{id}/assets"
    }
  ],
  rpc: [
    {
      title: "Latest Ledger Info",
      description: "Get the most recent Stellar ledger information",
      language: "python",
      code: `import requests
from datetime import datetime

def get_latest_ledger():
    """Fetch the latest Stellar ledger information"""
    
    url = "http://localhost:8080/api/rpc/ledger/latest"
    
    try:
        response = requests.get(url)
        response.raise_for_status()
        
        ledger = response.json()
        
        print(f"Ledger Sequence: {ledger['sequence']}")
        print(f"Timestamp: {ledger['timestamp']}")
        print(f"Transaction Count: {ledger['transaction_count']}")
        print(f"Operations Count: {ledger['operation_count']}")
        
        # Convert timestamp to readable format
        if 'timestamp' in ledger:
            dt = datetime.fromisoformat(ledger['timestamp'].replace('Z', '+00:00'))
            print(f"Local Time: {dt.strftime('%Y-%m-%d %H:%M:%S')}")
        
        return ledger
        
    except requests.exceptions.RequestException as e:
        print(f"Error: {e}")
        return None

# Usage
get_latest_ledger()`,
      endpoint: "GET /api/rpc/ledger/latest"
    },
    {
      title: "Recent Payments",
      description: "Fetch recent payment transactions from network",
      language: "javascript",
      code: `// Get recent payments with filtering
async function getRecentPayments(limit = 20) {
  try {
    const response = await fetch(
      'http://localhost:8080/api/rpc/payments?limit=' + limit
    );
    
    if (!response.ok) {
      throw new Error('HTTP error! status: ' + response.status);
    }
    
    const data = await response.json();
    const payments = data.payments || [];
    
    // Process payments
    const processedPayments = payments.map(payment => ({
      id: payment.id,
      from: payment.from,
      to: payment.to,
      amount: payment.amount,
      asset: payment.asset_code || 'XLM',
      timestamp: payment.created_at,
      successful: payment.successful
    }));
    
    console.log('Found ' + processedPayments.length + ' recent payments');
    
    // Show successful vs failed ratio
    const successful = processedPayments.filter(p => p.successful).length;
    const successRate = (successful / processedPayments.length * 100).toFixed(1);
    console.log('Success rate: ' + successRate + '%');
    
    return processedPayments;
    
  } catch (error) {
    console.error('Error fetching payments:', error);
  }
}

// Usage
getRecentPayments();`,
      endpoint: "GET /api/rpc/payments"
    },
    {
      title: "Account Payments",
      description: "Get all payments for a specific Stellar account",
      language: "python",
      code: `import requests

def get_account_payments(account_id, limit=10):
    """Get payments for a specific Stellar account"""
    
    url = f"http://localhost:8080/api/rpc/payments/account/{account_id}"
    params = {'limit': limit}
    
    try:
        response = requests.get(url, params=params)
        response.raise_for_status()
        
        data = response.json()
        payments = data.get('payments', [])
        
        print(f"Payments for account: {account_id}")
        print("=" * 50)
        
        for i, payment in enumerate(payments, 1):
            print(f"{i}. {payment.get('created_at', 'Unknown time')}")
            print(f"   From: {payment.get('from', 'Unknown')}")
            print(f"   To: {payment.get('to', 'Unknown')}")
            print(f"   Amount: {payment.get('amount', '0')} {payment.get('asset_code', 'XLM')}")
            print(f"   Status: {'✓ Success' if payment.get('successful') else '✗ Failed'}")
            print()
        
        return payments
        
    except requests.exceptions.RequestException as e:
        print(f"Error: {e}")
        return None

# Example usage
# account_id = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
# get_account_payments(account_id)`,
      endpoint: "GET /api/rpc/payments/account/{account_id}"
    }
  ]
};

const languageIcons: Record<string, React.ReactNode> = {
  javascript: <Code2 className="h-4 w-4" />,
  python: <Terminal className="h-4 w-4" />,
  curl: <Globe className="h-4 w-4" />
};

const languageColors: Record<string, string> = {
  javascript: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200',
  python: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
  curl: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
};

const CodeExamples = () => {
  const [selectedCategory, setSelectedCategory] = useState('corridors');
  const [selectedLanguage, setSelectedLanguage] = useState('all');
  const [copiedExample, setCopiedExample] = useState<string | null>(null);

  const copyToClipboard = async (code: string, exampleId: string) => {
    try {
      await navigator.clipboard.writeText(code);
      setCopiedExample(exampleId);
      setTimeout(() => setCopiedExample(null), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  const filteredExamples = codeExamples[selectedCategory]?.filter(example => 
    selectedLanguage === 'all' || example.language === selectedLanguage
  ) || [];

  return (
    <div className="container mx-auto px-4 py-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-2">Code Examples</h1>
        <p className="text-muted-foreground dark:text-gray-300">
          Ready-to-use code examples in multiple programming languages
        </p>
      </div>

      <div className="mb-6">
        <div className="flex flex-col sm:flex-row gap-4">
          <Select value={selectedCategory} onValueChange={setSelectedCategory}>
            <SelectTrigger className="w-full sm:w-[200px]">
              <SelectValue placeholder="Select category" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="corridors">
                <div className="flex items-center gap-2">
                  <Network className="h-4 w-4" />
                  Corridors
                </div>
              </SelectItem>
              <SelectItem value="anchors">
                <div className="flex items-center gap-2">
                  <Database className="h-4 w-4" />
                  Anchors
                </div>
              </SelectItem>
              <SelectItem value="rpc">
                <div className="flex items-center gap-2">
                  <Zap className="h-4 w-4" />
                  RPC
                </div>
              </SelectItem>
            </SelectContent>
          </Select>

          <Select value={selectedLanguage} onValueChange={setSelectedLanguage}>
            <SelectTrigger className="w-full sm:w-[150px]">
              <SelectValue placeholder="Language" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Languages</SelectItem>
              <SelectItem value="javascript">JavaScript</SelectItem>
              <SelectItem value="python">Python</SelectItem>
              <SelectItem value="curl">cURL</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      <div className="space-y-6">
        {filteredExamples.map((example, index) => {
          const exampleId = selectedCategory + '-' + index;
          const isCopied = copiedExample === exampleId;
          
          return (
            <Card key={exampleId}>
              <CardHeader>
                <div className="flex items-start justify-between">
                  <div>
                    <CardTitle className="flex items-center gap-2 mb-2">
                      {languageIcons[example.language]}
                      {example.title}
                    </CardTitle>
                    <CardDescription>{example.description}</CardDescription>
                    {example.endpoint && (
                      <Badge variant="outline" className="mt-2">
                        {example.endpoint}
                      </Badge>
                    )}
                  </div>
                  <Badge className={languageColors[example.language]}>
                    {example.language.toUpperCase()}
                  </Badge>
                </div>
              </CardHeader>
              <CardContent>
                <div className="relative">
                  <pre className="bg-gray-100 dark:bg-gray-800 p-4 rounded-lg overflow-x-auto">
                    <code className="text-sm">{example.code}</code>
                  </pre>
                  <Button
                    variant="outline"
                    size="sm"
                    className="absolute top-2 right-2 gap-1"
                    onClick={() => copyToClipboard(example.code, exampleId)}
                  >
                    {isCopied ? (
                      <>
                        <CheckCircle className="h-4 w-4" />
                        Copied!
                      </>
                    ) : (
                      <>
                        <Copy className="h-4 w-4" />
                        Copy
                      </>
                    )}
                  </Button>
                </div>
              </CardContent>
            </Card>
          );
        })}
      </div>

      {filteredExamples.length === 0 && (
        <Card>
          <CardContent className="p-8 text-center">
            <Code2 className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
            <h3 className="text-lg font-medium mb-2">No Examples Found</h3>
            <p className="text-muted-foreground dark:text-muted-foreground">
              Try selecting a different category or language
            </p>
          </CardContent>
        </Card>
      )}

      {/* Quick Reference */}
      <Card className="mt-8">
        <CardHeader>
          <CardTitle>Quick Reference</CardTitle>
          <CardDescription>
            Common API patterns and best practices
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <h4 className="font-medium mb-2">Base URL</h4>
              <code className="text-sm bg-gray-100 dark:bg-gray-800 p-2 rounded block">
                http://localhost:8080
              </code>
            </div>
            <div>
              <h4 className="font-medium mb-2">Authentication</h4>
              <p className="text-sm text-muted-foreground dark:text-muted-foreground">
                No API key required for public endpoints
              </p>
            </div>
            <div>
              <h4 className="font-medium mb-2">Response Format</h4>
              <p className="text-sm text-muted-foreground dark:text-muted-foreground">
                All responses are in JSON format
              </p>
            </div>
            <div>
              <h4 className="font-medium mb-2">Rate Limiting</h4>
              <p className="text-sm text-muted-foreground dark:text-muted-foreground">
                100 requests per minute per IP
              </p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
};

export default CodeExamples;
