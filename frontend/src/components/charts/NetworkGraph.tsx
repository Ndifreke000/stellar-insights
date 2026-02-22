"use client";

import React, { useCallback, useRef, useEffect, useState } from 'react';
import ForceGraph2D, { ForceGraphMethods } from 'react-force-graph-2d';
import { useSearchParams } from 'next/navigation';

interface Node {
    id: string;
    name: string;
    type: 'anchor' | 'asset';
    val: number;
    status?: string;
    address?: string;
    issuer?: string;
    fullName?: string;
    x?: number;
    y?: number;
}

interface Link {
    source: string | Node;
    target: string | Node;
    type: 'issuance' | 'corridor';
    value: number;
    health?: number;
    liquidity?: number;
}

interface GraphData {
    nodes: Node[];
    links: Link[];
}

interface NetworkGraphProps {
    data: GraphData;
}

const NetworkGraph: React.FC<NetworkGraphProps> = ({ data }) => {
    const fgRef = useRef<ForceGraphMethods>();
    const [hoverNode, setHoverNode] = useState<Node | null>(null);
    const [windowSize, setWindowSize] = useState({ width: 800, height: 600 });

    useEffect(() => {
        const updateSize = () => {
            setWindowSize({
                width: window.innerWidth - (window.innerWidth > 1024 ? 280 : 40), // Adjust for sidebar
                height: window.innerHeight - 200
            });
        };
        window.addEventListener('resize', updateSize);
        updateSize();
        return () => window.removeEventListener('resize', updateSize);
    }, []);

    const getLinkColor = (link: Link) => {
        if (link.type === 'issuance') return 'rgba(148, 163, 184, 0.2)'; // Light gray/slate

        // Corridor colors based on health
        if (link.health !== undefined) {
            if (link.health >= 90) return 'rgba(74, 222, 128, 0.4)'; // Green
            if (link.health >= 70) return 'rgba(250, 204, 21, 0.4)'; // Yellow
            return 'rgba(248, 113, 113, 0.4)'; // Red
        }
        return 'rgba(99, 102, 241, 0.4)'; // Default accent
    };

    const getNodeColor = (node: Node) => {
        if (node.type === 'anchor') return '#6366f1'; // Indigo/Accent
        return '#f43f5e'; // Rose/Asset
    };

    const paintNode = useCallback((node: Node, ctx: CanvasRenderingContext2D, globalScale: number) => {
        const label = node.name;
        const fontSize = 12 / globalScale;
        ctx.font = `${fontSize}px Inter, sans-serif`;
        const textWidth = ctx.measureText(label).width;
        const bckgDimensions = [textWidth, fontSize].map(n => n + fontSize * 0.2); // some padding

        // Draw shadow/glow if hovered
        if (hoverNode && hoverNode.id === node.id) {
            ctx.shadowBlur = 15;
            ctx.shadowColor = getNodeColor(node);
        }

        // Draw node shape
        ctx.fillStyle = getNodeColor(node);
        if (node.type === 'anchor') {
            // Rounded rect for anchors
            const r = 4;
            const x = node.x! - 6;
            const y = node.y! - 6;
            const w = 12;
            const h = 12;
            ctx.beginPath();
            ctx.moveTo(x + r, y);
            ctx.arcTo(x + w, y, x + w, y + h, r);
            ctx.arcTo(x + w, y + h, x, y + h, r);
            ctx.arcTo(x, y + h, x, y, r);
            ctx.arcTo(x, y, x + w, y, r);
            ctx.closePath();
        } else {
            // Circle for assets
            ctx.beginPath();
            ctx.arc(node.x!, node.y!, 4, 0, 2 * Math.PI, false);
        }
        ctx.fill();

        // Reset shadow
        ctx.shadowBlur = 0;

        // Draw text label
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillStyle = 'rgba(255, 255, 255, 0.8)';
        ctx.fillText(label, node.x!, node.y! + 8);
    }, [hoverNode]);

    return (
        <div className="relative w-full h-full glass rounded-3xl overflow-hidden border border-border/50">
            <ForceGraph2D
                ref={fgRef}
                graphData={data}
                width={windowSize.width}
                height={windowSize.height}
                backgroundColor="transparent"
                nodeCanvasObject={paintNode}
                nodePointerAreaPaint={(node, color, ctx) => {
                    ctx.fillStyle = color;
                    ctx.beginPath();
                    ctx.arc(node.x!, node.y!, 8, 0, 2 * Math.PI, false);
                    ctx.fill();
                }}
                linkColor={getLinkColor}
                linkDirectionalArrowLength={3}
                linkDirectionalArrowRelPos={1}
                linkCurvature={0.25}
                linkWidth={link => link.type === 'corridor' ? (link as any).value : 1}
                onNodeHover={setHoverNode}
                cooldownTicks={100}
            />

            {/* Legend */}
            <div className="absolute top-6 left-6 p-4 glass border border-white/10 rounded-2xl flex flex-col gap-3">
                <div className="flex items-center gap-2">
                    <div className="w-3 h-3 bg-accent rounded" />
                    <span className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground">Anchor</span>
                </div>
                <div className="flex items-center gap-2">
                    <div className="w-3 h-3 bg-rose-500 rounded-full" />
                    <span className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground">Asset</span>
                </div>
                <div className="h-px bg-white/5 my-1" />
                <div className="flex items-center gap-2">
                    <div className="w-6 h-0.5 bg-green-400/40" />
                    <span className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground">Healthy Corridor</span>
                </div>
                <div className="flex items-center gap-2">
                    <div className="w-6 h-0.5 bg-red-400/40" />
                    <span className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground">Degraded Corridor</span>
                </div>
            </div>

            {/* Hover Info */}
            {hoverNode && (
                <div className="absolute bottom-6 left-6 p-4 glass-dark border border-white/10 rounded-2xl min-w-[200px] animate-in fade-in slide-in-from-bottom-2 duration-300">
                    <div className="flex items-center justify-between mb-2">
                        <span className={`text-[10px] font-bold uppercase tracking-widest px-2 py-0.5 rounded ${hoverNode.type === 'anchor' ? 'bg-accent/20 text-accent' : 'bg-rose-500/20 text-rose-400'}`}>
                            {hoverNode.type}
                        </span>
                    </div>
                    <h4 className="font-bold text-lg mb-1">{hoverNode.name}</h4>
                    {hoverNode.type === 'anchor' ? (
                        <p className="text-xs text-muted-foreground font-mono break-all">{hoverNode.address}</p>
                    ) : (
                        <div>
                            <p className="text-xs text-muted-foreground mb-1">Full Path: {hoverNode.fullName}</p>
                            <div className="flex items-center gap-2 mt-2">
                                <div className="w-2 h-2 rounded-full bg-green-500" />
                                <span className="text-[10px] font-mono uppercase text-green-400">Trading Active</span>
                            </div>
                        </div>
                    )}
                </div>
            )}

            {/* Controls Overlay */}
            <div className="absolute top-6 right-6 flex flex-col gap-2">
                <button
                    onClick={() => fgRef.current?.zoomToFit(400)}
                    className="p-3 glass hover:bg-white/10 border border-white/10 rounded-xl transition-all active:scale-95 group"
                    title="Recenter View"
                >
                    <svg xmlns="http://www.w3.org/2000/svg" className="w-5 h-5 text-muted-foreground group-hover:text-foreground" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M3 12h7m-7 0a9 9 0 1 0 18 0 9 9 0 1 0-18 0z" /><path d="M12 3v7" /><path d="M12 21v-7" /><path d="M14 10l-2 2-2-2" /></svg>
                </button>
            </div>
        </div>
    );
};

export default NetworkGraph;
