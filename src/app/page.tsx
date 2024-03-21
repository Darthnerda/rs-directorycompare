"use client"
import Image from "next/image";
// import styles from "./page.module.css";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";

export default function Home() {
  const [dir1, setDir1] = useState<string>("")
  const [dir2, setDir2] = useState<string>("")

  function handleGetDir(dialogTitle: string, setter: React.Dispatch<React.SetStateAction<string>>) {
    console.log("trying");
    
    invoke<string>("choose_folder", { dialogTitle: dialogTitle })
      .then(result => setter(result))
      .catch(console.error)
  }

  return (
    <main>
      <div>
        <button onClick={() => console.log("hi there")}>test</button>
        <button onClick={() => handleGetDir("Set Directory 1", setDir1)}>Set Directory 1</button>
        <button onClick={() => handleGetDir("Set Directory 2", setDir2)}>Set Directory 2</button>
      </div>
    </main>
  );
}
