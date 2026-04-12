import { Loader } from "lucide-react";

interface SpinnerProps {
  size?: number;
  label?: string;
}

/** A centered spinner with optional label text. */
export default function Spinner({ size = 20, label }: SpinnerProps) {
  return (
    <div
      role="status"
      aria-live="polite"
      className="fade-in"
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: "10px",
        padding: "40px",
        color: "var(--color-text-secondary)",
        fontSize: "13px",
      }}
    >
      <Loader size={size} className="spinner" />
      {label && <span>{label}</span>}
    </div>
  );
}
