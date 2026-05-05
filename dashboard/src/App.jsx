import React, { useEffect, useRef, useState } from 'react';
import { createChart, ColorType } from 'lightweight-charts';
import { motion, AnimatePresence } from 'framer-motion';
import { Activity, ShieldCheck, Zap, TrendingUp, TrendingDown, Terminal, Info, AlertTriangle, Cpu, Layers, Link2 } from 'lucide-react';

const App = () => {
    const chartContainerRef = useRef();
    const chartRef = useRef();
    const seriesRef = useRef();
    const [data, setData] = useState(null);
    const [history, setHistory] = useState([]);
    const [status, setStatus] = useState('connecting');
    const [lastSignal, setLastSignal] = useState('FLAT');

    useEffect(() => {
        const chart = createChart(chartContainerRef.current, {
            layout: {
                background: { type: ColorType.Solid, color: 'transparent' },
                textColor: '#94a3b8',
            },
            grid: {
                vertLines: { color: 'rgba(255, 255, 255, 0.05)' },
                horzLines: { color: 'rgba(255, 255, 255, 0.05)' },
            },
            rightPriceScale: {
                borderColor: 'rgba(255, 255, 255, 0.1)',
            },
            timeScale: {
                borderColor: 'rgba(255, 255, 255, 0.1)',
                timeVisible: true,
                secondsVisible: true,
            },
            crosshair: {
                mode: 0,
                vertLine: { color: '#6366f1', labelBackgroundColor: '#6366f1' },
                horzLine: { color: '#6366f1', labelBackgroundColor: '#6366f1' },
            },
        });

        const series = chart.addCandlestickSeries({
            upColor: '#22c55e',
            downColor: '#ef4444',
            borderVisible: false,
            wickUpColor: '#22c55e',
            wickDownColor: '#ef4444',
        });

        chartRef.current = chart;
        seriesRef.current = series;

        const handleResize = () => {
            chart.applyOptions({ width: chartContainerRef.current.clientWidth });
        };

        window.addEventListener('resize', handleResize);

        // WebSocket Integration
        const socket = new WebSocket('ws://localhost:8000/ws');

        socket.onopen = () => setStatus('online');
        socket.onclose = () => setStatus('offline');
        socket.onmessage = (event) => {
            const msg = JSON.parse(event.data);
            setData(msg);
            
            // Update Chart
            const timestamp = Math.floor(new Date(msg.timestamp).getTime() / 1000);
            series.update({
                time: timestamp,
                open: msg.open,
                high: msg.high,
                low: msg.low,
                close: msg.price,
            });

            if (msg.signal !== 'FLAT') {
                setLastSignal(msg.signal);
            }

            setHistory(prev => [msg, ...prev].slice(0, 10));
        };

        return () => {
            window.removeEventListener('resize', handleResize);
            chart.remove();
            socket.close();
        };
    }, []);

    return (
        <div className="flex h-screen w-full bg-[#0a0a0c] p-4 gap-4 overflow-hidden">
            {/* Sidebar / Left Panel */}
            <div className="w-1/4 flex flex-col gap-4">
                {/* Header Card */}
                <div className="glass-card p-6 rounded-2xl">
                    <h1 className="text-2xl font-bold glimmer-text mb-2">QUIMERIA-HYPERION</h1>
                    <div className="flex items-center gap-2">
                        <div className={`w-2 h-2 rounded-full ${status === 'online' ? 'bg-green-500 animate-pulse' : 'bg-red-500'}`} />
                        <span className="text-xs uppercase tracking-widest text-slate-400 font-medium">System {status}</span>
                    </div>
                </div>

                {/* Performance Metrics */}
                <div className="glass-card p-6 rounded-2xl flex-1 flex flex-col gap-6">
                    <div>
                        <div className="flex justify-between items-center mb-4">
                            <span className="text-slate-400 text-sm font-medium flex items-center gap-2">
                                <Activity size={16} /> ADELIC STABILITY
                            </span>
                            <span className="text-indigo-400 font-bold mono">{(data?.stability * 100 || 0).toFixed(2)}%</span>
                        </div>
                        <div className="w-full bg-slate-800 h-1.5 rounded-full overflow-hidden">
                            <motion.div 
                                className="bg-indigo-500 h-full"
                                initial={{ width: 0 }}
                                animate={{ width: `${(data?.stability * 100 || 0)}%` }}
                             />
                        </div>
                    </div>

                    <div>
                        <div className="flex justify-between items-center mb-4">
                            <span className="text-slate-400 text-sm font-medium flex items-center gap-2">
                                <Zap size={16} /> ENERGY BIAS
                            </span>
                            <span className={`font-bold mono ${data?.bias > 0 ? 'text-green-500' : 'text-red-500'}`}>
                                {data?.bias?.toFixed(4) || '0.000'}
                            </span>
                        </div>
                        <div className="relative w-full bg-slate-800 h-1.5 rounded-full">
                             <motion.div 
                                className={`absolute h-full ${data?.bias > 0 ? 'bg-green-500 left-1/2' : 'bg-red-500 right-1/2'}`}
                                animate={{ width: `${Math.abs(data?.bias * 50) || 0}%` }}
                             />
                             <div className="absolute left-1/2 top-0 bottom-0 w-0.5 bg-slate-400/20" />
                        </div>
                    </div>

                    <div className="mt-4 p-4 rounded-xl bg-indigo-500/10 border border-indigo-500/20">
                        <div className="flex items-center gap-3">
                            <ShieldCheck className={data?.is_legal ? "text-green-400" : "text-amber-400"} />
                            <div>
                                <p className="text-xs text-slate-400 uppercase font-bold">Veto Status</p>
                                <p className="text-sm font-medium">
                                    {data?.is_legal ? "λ6 Displacement Clear" : "Volatility Limit Violation"}
                                </p>
                            </div>
                        </div>
                    </div>

                    {/* Adelic Pipeline Status */}
                    <div className="grid grid-cols-3 gap-2 mt-2">
                        <div className="bg-slate-900/50 p-2 rounded-lg border border-white/5 flex flex-col items-center">
                            <Cpu size={14} className="text-indigo-400 mb-1" />
                            <span className="text-[10px] text-slate-500 font-bold uppercase">UROL</span>
                            <div className={`w-1.5 h-1.5 rounded-full ${data?.adelic_active ? 'bg-green-500' : 'bg-slate-700'} mt-1`} />
                        </div>
                        <div className="bg-slate-900/50 p-2 rounded-lg border border-white/5 flex flex-col items-center">
                            <Layers size={14} className="text-indigo-400 mb-1" />
                            <span className="text-[10px] text-slate-500 font-bold uppercase">IPDA</span>
                            <div className={`w-1.5 h-1.5 rounded-full ${data?.adelic_active ? 'bg-green-500' : 'bg-slate-700'} mt-1`} />
                        </div>
                        <div className="bg-slate-900/50 p-2 rounded-lg border border-white/5 flex flex-col items-center">
                            <Link2 size={14} className="text-indigo-400 mb-1" />
                            <span className="text-[10px] text-slate-500 font-bold uppercase">AECABI</span>
                            <div className={`w-1.5 h-1.5 rounded-full ${data?.adelic_active ? 'bg-green-500' : 'bg-slate-700'} mt-1`} />
                        </div>
                    </div>

                    <div className="flex-1 mt-4 overflow-hidden flex flex-col">
                         <div className="flex items-center gap-2 text-xs font-bold text-slate-500 uppercase mb-4">
                            <Terminal size={14} /> Execution Logs
                         </div>
                         <div className="flex-1 overflow-y-auto space-y-3 pr-2">
                            <AnimatePresence initial={false}>
                                {history.map((log, idx) => (
                                    <motion.div 
                                        key={idx}
                                        initial={{ opacity: 0, x: -20 }}
                                        animate={{ opacity: 1, x: 0 }}
                                        className="text-[10px] mono border-l-2 border-indigo-500/30 pl-3 py-1 text-slate-400"
                                    >
                                        <span className="text-indigo-400">[{new Date(log.timestamp).toLocaleTimeString()}]</span> {log.signal} @ {log.price.toFixed(5)}
                                    </motion.div>
                                ))}
                            </AnimatePresence>
                         </div>
                    </div>
                </div>
            </div>

            {/* Main Content Area */}
            <div className="flex-1 flex flex-col gap-4">
                {/* Top Bar / Signal Panel */}
                <div className="h-32 glass-card rounded-2xl flex items-center px-10 justify-between">
                    <div className="flex flex-col">
                        <span className="text-xs font-bold text-slate-500 uppercase tracking-widest">Market Feed</span>
                        <span className="text-4xl font-bold mono">BTC/USDT</span>
                    </div>

                    <div className="flex gap-12">
                        <div className="flex flex-col items-center">
                            <span className="text-xs font-bold text-slate-500 uppercase mb-1">Current Price</span>
                            <span className="text-2xl font-bold mono">{data?.price?.toFixed(2) || '---'}</span>
                        </div>
                        <div className="flex flex-col items-center">
                             <div className={`text-4xl font-black italic tracking-tighter ${
                                lastSignal === 'BUY' ? 'text-green-500 led-buy' : 
                                lastSignal === 'SELL' ? 'text-red-500 led-sell' : 'text-slate-600'
                             } px-6 py-2 rounded-lg border border-current bg-current/5`}>
                                {lastSignal}
                             </div>
                             <span className="text-[10px] font-bold text-slate-500 uppercase mt-2">Active Signal</span>
                        </div>
                    </div>
                </div>

                {/* Chart Area */}
                <div className="flex-1 glass-card rounded-3xl overflow-hidden relative">
                    <div ref={chartContainerRef} className="w-full h-full" />
                    
                    {/* Floating Info Overlays */}
                    <div className="absolute top-6 left-6 flex gap-3 pointer-events-none">
                         <div className="bg-black/60 backdrop-blur-md px-4 py-2 rounded-full border border-white/10 flex items-center gap-2">
                            <div className="w-2 h-2 rounded-full bg-green-500" />
                            <span className="text-xs font-bold mono text-green-400">LIVE SYNC</span>
                         </div>
                         <div className="bg-black/60 backdrop-blur-md px-4 py-2 rounded-full border border-white/10 flex items-center gap-2">
                            <Activity size={14} className="text-indigo-400" />
                            <span className="text-xs font-bold mono">IPDA v4.2 Internal</span>
                         </div>
                    </div>
                </div>
            </div>
        </div>
    );
};

export default App;
