import qrcode from "qrcode-generator";

export function QRCode({
  value,
  size = 128,
}: {
  value: string;
  size?: number;
}) {
  const qr = qrcode(0, "M");
  qr.addData(value);
  qr.make();

  const cells = qr.getModuleCount();
  const cellSize = size / cells;

  const rects: { x: number; y: number }[] = [];
  for (let y = 0; y < cells; y++) {
    for (let x = 0; x < cells; x++) {
      if (qr.isDark(y, x)) rects.push({ x, y });
    }
  }

  return (
    <svg
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      role="img"
      aria-label={`QR code for ${value}`}
      className="text-[var(--color-text)] bg-white p-1 rounded"
    >
      {rects.map((r) => (
        <rect
          key={`${r.x}-${r.y}`}
          x={r.x * cellSize}
          y={r.y * cellSize}
          width={cellSize}
          height={cellSize}
          fill="currentColor"
        />
      ))}
    </svg>
  );
}
