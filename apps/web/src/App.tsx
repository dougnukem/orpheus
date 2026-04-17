import { Navigate, Route, Routes } from "react-router-dom";
import ScanPage from "@/routes/ScanPage";
import MnemonicPage from "@/routes/MnemonicPage";
import ResultsPage from "@/routes/ResultsPage";
import WalletPage from "@/routes/WalletPage";
import AddressPage from "@/routes/AddressPage";
import ImportPage from "@/routes/ImportPage";

export default function App() {
  return (
    <div className="min-h-screen bg-[var(--color-bg)] text-[var(--color-text)]">
      <Routes>
        <Route path="/" element={<Navigate to="/scan" replace />} />
        <Route path="/scan" element={<ScanPage />} />
        <Route path="/mnemonic" element={<MnemonicPage />} />
        <Route path="/results" element={<ResultsPage />} />
        <Route path="/results/:walletId" element={<WalletPage />} />
        <Route path="/results/:walletId/:address" element={<AddressPage />} />
        <Route path="/import/:keyId" element={<ImportPage />} />
        <Route path="*" element={<Navigate to="/scan" replace />} />
      </Routes>
    </div>
  );
}
