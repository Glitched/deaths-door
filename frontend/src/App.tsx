import { BrowserRouter, Routes, Route } from "react-router-dom";
import { OverlayPage } from "@/pages/OverlayPage";

function HomePage() {
  return (
    <div className="min-h-screen bg-background text-foreground flex items-center justify-center">
      <h1 className="text-4xl font-bold">Death's Door</h1>
    </div>
  );
}

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<HomePage />} />
        <Route path="/overlay" element={<OverlayPage />} />
      </Routes>
    </BrowserRouter>
  );
}

export default App;
