"use client";

// Dependency-free SVG sparkline.
export default function Sparkline({
  data, width = 220, height = 48, stroke = "#6366f1",
}: { data: number[]; width?: number; height?: number; stroke?: string }) {
  if (!data || data.length === 0) {
    return <svg width={width} height={height} />;
  }
  const max = Math.max(...data, 1);
  const min = Math.min(...data, 0);
  const dx = data.length > 1 ? width / (data.length - 1) : width;
  const range = max - min || 1;
  const pts = data.map((v, i) => {
    const x = i * dx;
    const y = height - ((v - min) / range) * (height - 4) - 2;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
  const area = `0,${height} ${pts} ${width},${height}`;
  return (
    <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none">
      <polygon points={area} fill={stroke} fillOpacity="0.10" />
      <polyline points={pts} fill="none" stroke={stroke} strokeWidth="1.5" />
    </svg>
  );
}
