import React, { useEffect } from "react";
import mermaid from "mermaid";

mermaid.initialize({
  startOnLoad: true,
  theme: 'neutral',
  securityLevel: 'loose',
});

const Mermaid = ({ chart }) => {
  useEffect(() => {
    mermaid.run();
  }, []);
  
  return <div className="mermaid" style={{textAlign: "center"}}>{chart}</div>;
};

export default Mermaid;