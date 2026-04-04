import { useEffect, useState } from "react";
import { healthCheck } from "../../lib/api";

export default function InboxView() {
  const [status, setStatus] = useState<string>("Connecting...");

  useEffect(() => {
    healthCheck()
      .then(setStatus)
      .catch((err) => setStatus(`Error: ${err}`));
  }, []);

  return (
    <div className="flex items-center justify-center h-full">
      <p style={{ color: "var(--color-text-secondary)" }}>{status}</p>
    </div>
  );
}
