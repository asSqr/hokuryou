import { useState } from "react";
import NotebookHeader from "./notebook-header/notebook-header";
import NotebookMiddle from "./notebook-middle/notebook-middle";


export default function Notebook() {
  const [source, setSource] = useState<string>("");

  return (
    <div>
      <NotebookHeader />
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          justifyContent: "center",
          alignItems: "top",
          height: "calc(100vh - 65px)",
          gap: "0px",
        }}
      >
        <NotebookMiddle source={source} />
      </div>
    </div>
  );
}
