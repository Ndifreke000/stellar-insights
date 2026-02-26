'use client';

import React, { useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { 
  Play, 
  Copy, 
  CheckCircle, 
  XCircle, 
  Clock,
  Database,
  Network,
  Zap,
  BookOpen
} from 'lucide-react';

interface APIEndpoint {
  method: 'GET' | 'POST' | 'PUT';
  path: string;
  description: string;
  parameters?: Array<{
    name: string;
    type: string;
    required: boolean;
    description: string;
    default?: string;
  }>;
  example?: string;
}

const apiEndpoints: Record<string, APIEndpoint[]> = {
  corridors: [
    {
      method: 'GET',
      path: '/api/corridors',
      description: 'List all payment corridors with success rates and volume metrics',
      parameters: [
        {
          name: 'sort_by',
          type: 'string',
          required: false,
          description: 'Sort by success_rate or volume',
          default: 'success_rate'
        },
        {
          name: 'limit',
          type: 'integer',
          required: false,
          description: 'Maximum number of results to return',
          default: '50'
        },
        {
          name: 'offset',
          type: 'integer',
          required: false,
          description: 'Number of results to skip for pagination',
          default: '0'
        }
      ],
      example: 'curl -X GET "http://localhost:8080/api/corridors?sort_by=success_rate&limit=10"'
    },
    {
      method: 'GET',
      path: '/api/corridors/{asset_pair}',
      description: 'Get detailed metrics for a specific asset pair corridor',
      parameters: [
        {
          name: 'asset_pair',
          type: 'string',
          required: true,
          description: 'Asset pair in format ASSET:ISSUER->ASSET:ISSUER'
        }
      ],
      example: 'curl -X GET "http://localhost:8080/api/corridors/USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN->EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2"'
    }
  ],
  anchors: [
    {
      method: 'GET',
      path: '/api/anchors',
      description: 'List all anchors with reliability metrics',
      parameters: [
        {
          name: 'limit',
          type: 'integer',
          required: false,
          description: 'Maximum number of results to return',
          default: '50'
        }
      ],
      example: 'curl -X GET "http://localhost:8080/api/anchors"'
    },
    {
      method: 'GET',
      path: '/api/anchors/{id}',
      description: 'Get detailed information about a specific anchor',
      parameters: [
        {
          name: 'id',
          type: 'integer',
          required: true,
          description: 'Anchor ID'
        }
      ],
      example: 'curl -X GET "http://localhost:8080/api/anchors/1"'
    },
    {
      method: 'GET',
      path: '/api/anchors/{id}/assets',
      description: 'Get asset portfolio for a specific anchor',
      parameters: [
        {
          name: 'id',
          type: 'integer',
          required: true,
          description: 'Anchor ID'
        }
      ],
      example: 'curl -X GET "http://localhost:8080/api/anchors/1/assets"'
    }
  ],
  rpc: [
    {
      method: 'GET',
      path: '/api/rpc/health',
      description: 'Check RPC service health',
      example: 'curl -X GET "http://localhost:8080/api/rpc/health"'
    },
    {
      method: 'GET',
      path: '/api/rpc/ledger/latest',
      description: 'Get the latest Stellar ledger information',
      example: 'curl -X GET "http://localhost:8080/api/rpc/ledger/latest"'
    },
    {
      method: 'GET',
      path: '/api/rpc/payments',
      description: 'Fetch recent payment transactions',
      parameters: [
        {
          name: 'limit',
          type: 'integer',
          required: false,
          description: 'Maximum number of payments to return',
          default: '10'
        }
      ],
      example: 'curl -X GET "http://localhost:8080/api/rpc/payments?limit=10"'
    },
    {
      method: 'GET',
      path: '/api/rpc/payments/account/{account_id}',
      description: 'Get payments for a specific account',
      parameters: [
        {
          name: 'account_id',
          type: 'string',
          required: true,
          description: 'Stellar account ID'
        }
      ],
      example: 'curl -X GET "http://localhost:8080/api/rpc/payments/account/GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"'
    }
  ]
};

const APIPlayground = () => {
  const [selectedCategory, setSelectedCategory] = useState('corridors');
  const [selectedEndpoint, setSelectedEndpoint] = useState<APIEndpoint | null>(null);
  const [parameters, setParameters] = useState<Record<string, string>>({});
  const [response, setResponse] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [copied, setCopied] = useState(false);

  const handleEndpointSelect = (endpoint: APIEndpoint) => {
    setSelectedEndpoint(endpoint);
    setResponse('');
    
    // Initialize parameters with defaults
    const params: Record<string, string> = {};
    endpoint.parameters?.forEach(param => {
      if (param.default) {
        params[param.name] = param.default;
      }
    });
    setParameters(params);
  };

  const handleParameterChange = (name: string, value: string) => {
    setParameters(prev => ({ ...prev, [name]: value }));
  };

  const buildRequestUrl = () => {
    if (!selectedEndpoint) return '';
    
    let url = `http://localhost:8080${selectedEndpoint.path}`;
    
    // Replace path parameters
    Object.entries(parameters).forEach(([key, value]) => {
      if (selectedEndpoint.path.includes(`{${key}}`)) {
        url = url.replace(`{${key}}`, value);
      }
    });
    
    // Add query parameters
    const queryParams = Object.entries(parameters)
      .filter(([key]) => !selectedEndpoint.path.includes(`{${key}}`))
      .map(([key, value]) => `${key}=${encodeURIComponent(value)}`)
      .join('&');
    
    if (queryParams) {
      url += `?${queryParams}`;
    }
    
    return url;
  };

  const executeRequest = async () => {
    if (!selectedEndpoint) return;
    
    setLoading(true);
    setResponse('');
    
    try {
      const url = buildRequestUrl();
      const fetchOptions: RequestInit = {
        method: selectedEndpoint.method,
        headers: {
          'Content-Type': 'application/json',
        },
      };
      
      const startTime = Date.now();
      const res = await fetch(url, fetchOptions);
      const endTime = Date.now();
      const responseTime = endTime - startTime;
      
      let responseText = '';
      try {
        const data = await res.json();
        responseText = JSON.stringify(data, null, 2);
      } catch {
        responseText = await res.text();
      }
      
      setResponse(`Status: ${res.status} ${res.statusText}
Response Time: ${responseTime}ms

${responseText}`);
    } catch (error) {
      setResponse(`Error: ${error instanceof Error ? error.message : 'Unknown error'}`);
    } finally {
      setLoading(false);
    }
  };

  const copyToClipboard = () => {
    const url = buildRequestUrl();
    navigator.clipboard.writeText(url);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const getMethodColor = (method: string) => {
    switch (method) {
      case 'GET': return 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200';
      case 'POST': return 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200';
      case 'PUT': return 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200';
      default: return 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200';
    }
  };

  const getCategoryIcon = (category: string) => {
    switch (category) {
      case 'corridors': return <Network className="h-4 w-4" />;
      case 'anchors': return <Database className="h-4 w-4" />;
      case 'rpc': return <Zap className="h-4 w-4" />;
      default: return <BookOpen className="h-4 w-4" />;
    }
  };

  return (
    <div className="container mx-auto px-4 py-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-2">API Playground</h1>
        <p className="text-muted-foreground dark:text-gray-300">
          Test our API endpoints directly in your browser. No setup required.
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Endpoint Selection */}
        <div className="lg:col-span-1">
          <Card>
            <CardHeader>
              <CardTitle>Endpoints</CardTitle>
              <CardDescription>Select an endpoint to test</CardDescription>
            </CardHeader>
            <CardContent>
              <Tabs value={selectedCategory} onValueChange={setSelectedCategory}>
                <TabsList className="grid w-full grid-cols-3">
                  <TabsTrigger value="corridors">Corridors</TabsTrigger>
                  <TabsTrigger value="anchors">Anchors</TabsTrigger>
                  <TabsTrigger value="rpc">RPC</TabsTrigger>
                </TabsList>
                
                {Object.entries(apiEndpoints).map(([category, endpoints]) => (
                  <TabsContent key={category} value={category} className="mt-4">
                    <div className="space-y-2">
                      {endpoints.map((endpoint, index) => (
                        <div
                          key={index}
                          className={`p-3 border rounded-lg cursor-pointer transition-colors ${
                            selectedEndpoint === endpoint
                              ? 'border-blue-500 bg-blue-50 dark:bg-blue-950'
                              : 'hover:bg-gray-50 dark:hover:bg-gray-800'
                          }`}
                          onClick={() => handleEndpointSelect(endpoint)}
                        >
                          <div className="flex items-center gap-2 mb-1">
                            <Badge className={`text-xs ${getMethodColor(endpoint.method)}`}>
                              {endpoint.method}
                            </Badge>
                            <span className="text-sm font-mono">{endpoint.path}</span>
                          </div>
                          <p className="text-xs text-muted-foreground dark:text-muted-foreground">
                            {endpoint.description}
                          </p>
                        </div>
                      ))}
                    </div>
                  </TabsContent>
                ))}
              </Tabs>
            </CardContent>
          </Card>
        </div>

        {/* Request Builder */}
        <div className="lg:col-span-2 space-y-6">
          {selectedEndpoint && (
            <>
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    {getCategoryIcon(selectedCategory)}
                    Request Builder
                  </CardTitle>
                  <CardDescription>
                    Configure parameters and test the endpoint
                  </CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div className="flex items-center gap-2 p-3 bg-gray-100 dark:bg-gray-800 rounded-lg">
                    <Badge className={getMethodColor(selectedEndpoint.method)}>
                      {selectedEndpoint.method}
                    </Badge>
                    <code className="flex-1 text-sm font-mono">
                      {buildRequestUrl()}
                    </code>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={copyToClipboard}
                      className="gap-1"
                    >
                      {copied ? <CheckCircle className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
                      {copied ? 'Copied!' : 'Copy'}
                    </Button>
                  </div>

                  {selectedEndpoint.parameters && selectedEndpoint.parameters.length > 0 && (
                    <div className="space-y-3">
                      <h4 className="font-medium">Parameters</h4>
                      {selectedEndpoint.parameters.map((param) => (
                        <div key={param.name} className="space-y-1">
                          <Label htmlFor={param.name}>
                            {param.name}
                            {param.required && <span className="text-red-500 ml-1">*</span>}
                          </Label>
                          {param.type === 'integer' ? (
                            <Input
                              id={param.name}
                              type="number"
                              value={parameters[param.name] || ''}
                              onChange={(e) => handleParameterChange(param.name, e.target.value)}
                              placeholder={param.default}
                            />
                          ) : (
                            <Input
                              id={param.name}
                              type="text"
                              value={parameters[param.name] || ''}
                              onChange={(e) => handleParameterChange(param.name, e.target.value)}
                              placeholder={param.default}
                            />
                          )}
                          <p className="text-xs text-muted-foreground dark:text-muted-foreground">
                            {param.description}
                          </p>
                        </div>
                      ))}
                    </div>
                  )}

                  <Button
                    onClick={executeRequest}
                    disabled={loading}
                    className="w-full gap-2"
                  >
                    {loading ? (
                      <>
                        <Clock className="h-4 w-4 animate-spin" />
                        Executing...
                      </>
                    ) : (
                      <>
                        <Play className="h-4 w-4" />
                        Execute Request
                      </>
                    )}
                  </Button>
                </CardContent>
              </Card>

              {/* Response */}
              {response && (
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2">
                      {response.includes('Error') ? (
                        <XCircle className="h-5 w-5 text-red-500" />
                      ) : (
                        <CheckCircle className="h-5 w-5 text-green-500" />
                      )}
                      Response
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <Textarea
                      value={response}
                      readOnly
                      className="font-mono text-sm min-h-[200px]"
                    />
                  </CardContent>
                </Card>
              )}

              {/* Example */}
              {selectedEndpoint.example && (
                <Card>
                  <CardHeader>
                    <CardTitle>Example</CardTitle>
                    <CardDescription>Ready-to-use example command</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <pre className="bg-gray-100 dark:bg-gray-800 p-4 rounded-lg overflow-x-auto">
                      <code>{selectedEndpoint.example}</code>
                    </pre>
                  </CardContent>
                </Card>
              )}
            </>
          )}

          {!selectedEndpoint && (
            <Card>
              <CardContent className="p-8 text-center">
                <BookOpen className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
                <h3 className="text-lg font-medium mb-2">Select an Endpoint</h3>
                <p className="text-muted-foreground dark:text-muted-foreground">
                  Choose an endpoint from the left to start testing the API
                </p>
              </CardContent>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
};

export default APIPlayground;
