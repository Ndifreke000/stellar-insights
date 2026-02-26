import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { 
  BookOpen, 
  Code2, 
  Play, 
  Rocket, 
  Database, 
  Network,
  ArrowRight,
  CheckCircle,
  Clock,
  Zap
} from 'lucide-react';

const APIDocumentationPortal = () => {
  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-50 to-blue-50 dark:from-slate-900 dark:to-slate-800">
      {/* Hero Section */}
      <div className="container mx-auto px-4 py-16">
        <div className="text-center mb-16">
          <div className="flex justify-center mb-6">
            <div className="p-3 bg-blue-100 dark:bg-blue-900 rounded-full">
              <BookOpen className="h-12 w-12 text-blue-600 dark:text-link-primary" />
            </div>
          </div>
          <h1 className="text-5xl font-bold text-gray-900 dark:text-white mb-6">
            Stellar Insights API
          </h1>
          <p className="text-xl text-muted-foreground dark:text-gray-300 max-w-3xl mx-auto mb-8">
            Comprehensive REST API for Stellar network analytics. Access payment reliability metrics, 
            corridor health scores, anchor performance data, and real-time network insights.
          </p>
          <div className="flex gap-4 justify-center">
            <Button size="lg" className="gap-2">
              <Rocket className="h-5 w-5" />
              Get Started
            </Button>
            <Button variant="outline" size="lg" className="gap-2">
              <Play className="h-5 w-5" />
              Try Playground
            </Button>
          </div>
        </div>

        {/* Quick Stats */}
        <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-16">
          <Card>
            <CardContent className="p-6 text-center">
              <Database className="h-8 w-8 text-blue-600 mx-auto mb-2" />
              <div className="text-2xl font-bold">15+</div>
              <div className="text-sm text-muted-foreground dark:text-muted-foreground">API Endpoints</div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-6 text-center">
              <Network className="h-8 w-8 text-green-600 mx-auto mb-2" />
              <div className="text-2xl font-bold">Real-time</div>
              <div className="text-sm text-muted-foreground dark:text-muted-foreground">Stellar Data</div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-6 text-center">
              <Zap className="h-8 w-8 text-purple-600 mx-auto mb-2" />
              <div className="text-2xl font-bold">&lt;100ms</div>
              <div className="text-sm text-muted-foreground dark:text-muted-foreground">Response Time</div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-6 text-center">
              <CheckCircle className="h-8 w-8 text-emerald-600 mx-auto mb-2" />
              <div className="text-2xl font-bold">99.9%</div>
              <div className="text-sm text-muted-foreground dark:text-muted-foreground">Uptime</div>
            </CardContent>
          </Card>
        </div>

        {/* API Categories */}
        <Tabs defaultValue="corridors" className="mb-16">
          <TabsList className="grid w-full grid-cols-4">
            <TabsTrigger value="corridors">Corridors</TabsTrigger>
            <TabsTrigger value="anchors">Anchors</TabsTrigger>
            <TabsTrigger value="rpc">RPC</TabsTrigger>
            <TabsTrigger value="analytics">Analytics</TabsTrigger>
          </TabsList>

          <TabsContent value="corridors" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Network className="h-5 w-5" />
                  Payment Corridors API
                </CardTitle>
                <CardDescription>
                  Analyze payment reliability and performance between asset pairs
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div className="p-4 border rounded-lg">
                      <code className="text-sm bg-gray-100 dark:bg-gray-800 p-2 rounded block mb-2">
                        GET /api/corridors
                      </code>
                      <p className="text-sm text-muted-foreground dark:text-muted-foreground">
                        List all payment corridors with success rates and volume metrics
                      </p>
                    </div>
                    <div className="p-4 border rounded-lg">
                      <code className="text-sm bg-gray-100 dark:bg-gray-800 p-2 rounded block mb-2">
                        GET /api/corridors/{'{asset_pair}'}
                      </code>
                      <p className="text-sm text-muted-foreground dark:text-muted-foreground">
                        Get detailed metrics for a specific asset pair corridor
                      </p>
                    </div>
                  </div>
                  <div className="flex gap-2">
                    <Badge variant="secondary">Pagination</Badge>
                    <Badge variant="secondary">Sorting</Badge>
                    <Badge variant="secondary">Real-time</Badge>
                  </div>
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="anchors" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Database className="h-5 w-5" />
                  Anchors API
                </CardTitle>
                <CardDescription>
                  Monitor anchor performance, reliability scores, and asset portfolios
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div className="p-4 border rounded-lg">
                      <code className="text-sm bg-gray-100 dark:bg-gray-800 p-2 rounded block mb-2">
                        GET /api/anchors
                      </code>
                      <p className="text-sm text-muted-foreground dark:text-muted-foreground">
                        List all anchors with reliability metrics
                      </p>
                    </div>
                    <div className="p-4 border rounded-lg">
                      <code className="text-sm bg-gray-100 dark:bg-gray-800 p-2 rounded block mb-2">
                        GET /api/anchors/{'{id}'}/assets
                      </code>
                      <p className="text-sm text-muted-foreground dark:text-muted-foreground">
                        Get asset portfolio for a specific anchor
                      </p>
                    </div>
                  </div>
                  <div className="flex gap-2">
                    <Badge variant="secondary">Performance Tracking</Badge>
                    <Badge variant="secondary">Asset Analysis</Badge>
                  </div>
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="rpc" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Zap className="h-5 w-5" />
                  Stellar RPC API
                </CardTitle>
                <CardDescription>
                  Direct access to Stellar network data and real-time information
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div className="p-4 border rounded-lg">
                      <code className="text-sm bg-gray-100 dark:bg-gray-800 p-2 rounded block mb-2">
                        GET /api/rpc/ledger/latest
                      </code>
                      <p className="text-sm text-muted-foreground dark:text-muted-foreground">
                        Get the latest Stellar ledger information
                      </p>
                    </div>
                    <div className="p-4 border rounded-lg">
                      <code className="text-sm bg-gray-100 dark:bg-gray-800 p-2 rounded block mb-2">
                        GET /api/rpc/payments
                      </code>
                      <p className="text-sm text-muted-foreground dark:text-muted-foreground">
                        Fetch recent payment transactions
                      </p>
                    </div>
                  </div>
                  <div className="flex gap-2">
                    <Badge variant="secondary">Live Data</Badge>
                    <Badge variant="secondary">Blockchain Access</Badge>
                  </div>
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="analytics" className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <BookOpen className="h-5 w-5" />
                  Analytics API
                </CardTitle>
                <CardDescription>
                  Advanced analytics endpoints for deep network insights
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div className="p-4 border rounded-lg">
                      <code className="text-sm bg-gray-100 dark:bg-gray-800 p-2 rounded block mb-2">
                        GET /api/analytics/health
                      </code>
                      <p className="text-sm text-muted-foreground dark:text-muted-foreground">
                        Overall network health metrics
                      </p>
                    </div>
                    <div className="p-4 border rounded-lg">
                      <code className="text-sm bg-gray-100 dark:bg-gray-800 p-2 rounded block mb-2">
                        GET /api/analytics/trends
                      </code>
                      <p className="text-sm text-muted-foreground dark:text-muted-foreground">
                        Historical trends and patterns
                      </p>
                    </div>
                  </div>
                  <div className="flex gap-2">
                    <Badge variant="secondary">Historical Data</Badge>
                    <Badge variant="secondary">Predictive Analytics</Badge>
                  </div>
                </div>
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>

        {/* Quick Start */}
        <Card className="mb-16">
          <CardHeader>
            <CardTitle>Quick Start</CardTitle>
            <CardDescription>
              Get up and running in minutes with our simple API
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="flex items-center gap-4">
                <div className="flex-shrink-0 w-8 h-8 bg-blue-100 dark:bg-blue-900 rounded-full flex items-center justify-center text-sm font-medium">
                  1
                </div>
                <div>
                  <h4 className="font-medium">Get API Access</h4>
                  <p className="text-sm text-muted-foreground dark:text-muted-foreground">
                    No API key required for public endpoints
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-4">
                <div className="flex-shrink-0 w-8 h-8 bg-blue-100 dark:bg-blue-900 rounded-full flex items-center justify-center text-sm font-medium">
                  2
                </div>
                <div>
                  <h4 className="font-medium">Make Your First Request</h4>
                  <p className="text-sm text-muted-foreground dark:text-muted-foreground">
                    Try our corridors endpoint to see payment reliability data
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-4">
                <div className="flex-shrink-0 w-8 h-8 bg-blue-100 dark:bg-blue-900 rounded-full flex items-center justify-center text-sm font-medium">
                  3
                </div>
                <div>
                  <h4 className="font-medium">Integrate</h4>
                  <p className="text-sm text-muted-foreground dark:text-muted-foreground">
                    Use our code examples to integrate into your application
                  </p>
                </div>
              </div>
            </div>
            <div className="mt-6">
              <Button className="w-full gap-2">
                View Code Examples
                <ArrowRight className="h-4 w-4" />
              </Button>
            </div>
          </CardContent>
        </Card>

        {/* Code Example Preview */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Code2 className="h-5 w-5" />
              Example Request
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div>
                <h4 className="font-medium mb-2">JavaScript/TypeScript</h4>
                <pre className="bg-gray-100 dark:bg-gray-800 p-4 rounded-lg overflow-x-auto">
                  <code>{`// Fetch top corridors by success rate
const response = await fetch(
  'http://localhost:8080/api/corridors?sort_by=success_rate&limit=10'
);
const data = await response.json();
console.log('Top corridors:', data.corridors);`}</code>
                </pre>
              </div>
              <div>
                <h4 className="font-medium mb-2">cURL</h4>
                <pre className="bg-gray-100 dark:bg-gray-800 p-4 rounded-lg overflow-x-auto">
                  <code>{`curl -X GET "http://localhost:8080/api/corridors?sort_by=success_rate&limit=10"`}</code>
                </pre>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
};

export default APIDocumentationPortal;
