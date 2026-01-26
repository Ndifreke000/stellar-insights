import { BrowserRouter, Routes, Route } from "react-router-dom";
import { WalletProvider } from "@/contexts/WalletContext";
import Landing from "@/pages/Landing";
import Corridors from "@/pages/Corridors";
import CorridorDetail from "@/pages/CorridorDetail";

function App() {
  return (
    <WalletProvider>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<Landing />} />
          <Route path="/corridors" element={<Corridors />} />
          <Route path="/corridors/:id" element={<CorridorDetail />} />
        </Routes>
      </BrowserRouter>
    </WalletProvider>
  );
}

export default App;
