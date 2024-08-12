import Image from "next/image";
import { Tasks } from "@/app/tasks"
export default function Home() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-between p-24">
     <div>
      <Tasks/>
      Hi</div> 
    </main>
  );
}
