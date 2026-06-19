import { useState } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { Header } from "@/components/layout/Header";
import type { Provider } from "@/types";
import ScanPage from "@/routes/ScanPage";
import MnemonicPage from "@/routes/MnemonicPage";
import ResultsPage from "@/routes/ResultsPage";
import WalletPage from "@/routes/WalletPage";
import AddressPage from "@/routes/AddressPage";
import ImportPage from "@/routes/ImportPage";

export default function App() {
  const [provider, setProvider] = useState<Provider>("blockstream");
  return (
    <div className="min-h-screen bg-[var(--color-bg)] text-[var(--color-text)]">
      <Header provider={provider} onProviderChange={setProvider} />
      <main className="max-w-[1200px] mx-auto px-6 py-6">
        <Routes>
          <Route path="/" element={<Navigate to="/scan" replace />} />
          <Route path="/scan" element={<ScanPage provider={provider} />} />
          <Route path="/mnemonic" element={<MnemonicPage />} />
          <Route path="/results" element={<ResultsPage />} />
          <Route path="/results/:walletId" element={<WalletPage />} />
          <Route
            path="/results/:walletId/:address"
            element={<AddressPage provider={provider} />}
          />
          <Route path="/import/:keyId" element={<ImportPage />} />
          <Route path="*" element={<Navigate to="/scan" replace />} />
        </Routes>
      </main>
    </div>
  );
}
