'use client'

import { Task } from "@/bindings/Task"
import { invoke } from "@tauri-apps/api"
import { useEffect, useState } from "react"

let colors = {
    ["default"]: "bg-gray-500",
    ["red"]: "bg-red-500",
    ["orange"]: "bg-orange-500",
    ["yellow"]: "bg-yellow-500",
    ["green"]: "bg-green-500",
    ["blue"]: "bg-blue-500",
    ["purple"]: "bg-purple-500",
    ["pink"]: "bg-pink-500",
    ["brown"]: "bg-brown-500",
    ["gray"]: "bg-gray-500",
}

export function Tasks() {
    const [tasks, setTasks] = useState<Array<Task>>([])

  useEffect(() => {
      invoke<Array<Task>>("tasks").then(result => {
        console.log(result)      
        setTasks(result)
  }).catch(console.error)
  })
return <div>{tasks.map(x =><div className={`${colors[x.status.color as any]}`} key={x.id}>
    {x.class.map(y => {
        const icon = y.icon
        switch(icon?.type) {
            case "emoji": return <span>{icon.emoji}</span>
            case "external": return <img src={icon.external.url} alt={icon.external.url}/>
            case "file": return <img src={icon.file.url} alt={icon.file.url}/>
        }
    })}   
    {x.due_date.start}
    {x.name}</div>)}</div>
}