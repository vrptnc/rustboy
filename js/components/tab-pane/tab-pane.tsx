import React, {Fragment, ReactNode, useState} from "react";
import {WebEmulator} from "../../../pkg/rustboy";
import {FaTableCells, FaVectorSquare} from "react-icons/fa6";

import './tab-pane.scss'
import {TileAtlas} from "../tile-atlas/tile-atlas";
import {ObjectAtlas} from "../object-atlas/object-atlas";

export interface TabBarProps {
  emulator: WebEmulator | undefined
}

enum Tab {
  TILE_MEMORY,
  OBJECT_MEMORY
}

interface TabConfig {
  icon: ReactNode
  tab: Tab
}


export const TabPane = ({ emulator }: TabBarProps) => {
  const [activeTab, setActiveTab] = useState<Tab>()

  const Content = () => {
    if (activeTab === Tab.TILE_MEMORY) {
      return <div className="content"><TileAtlas emulator={ emulator }/></div>
    } else if (activeTab === Tab.OBJECT_MEMORY) {
      return <div className="content"><ObjectAtlas emulator={ emulator }/></div>
    }
    return <Fragment/>
  }

  const Tabs = () => {
    const TABS: Array<TabConfig> = [
      {
        icon: <FaTableCells title="Tile Memory" size={ 20 }/>,
        tab: Tab.TILE_MEMORY
      },
      {
        icon: <FaVectorSquare title="Object Memory" size={ 20 }/>,
        tab: Tab.OBJECT_MEMORY
      }
    ]

    const getTab = ({ icon, tab }: TabConfig) => <div
      className={ 'tab' + (activeTab === tab ? ' active' : '') }
      onClick={ () => setActiveTab(activeTab === tab ? undefined : tab) }>
      {
        icon
      }
    </div>

    return <div className="tabs">
      {
        TABS.map((tabConfig) => getTab(tabConfig))
      }
    </div>
  }

  return emulator ? <div className="tab-pane">
    <Content/>
    <Tabs/>
  </div> : <Fragment/>
}