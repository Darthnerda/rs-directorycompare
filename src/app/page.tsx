"use client"
import Image from "next/image";
// import styles from "./page.module.css";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { Box, Button, Container, Divider, Paper, Tooltip, Typography } from "@mui/material";
import { DataGrid, GridColDef, GridValueFormatterParams, GridValueGetterParams } from '@mui/x-data-grid';
import { humanFileSize } from "@/utilities";

type FileInfo = {
  filename: string,
  path: string,
  should_copy: boolean,
  size: number
}

type CompInfo = {
  left: FileInfo[]
  right: FileInfo[]
  left_path: string
  right_path: string
}

function CopyGrid({compInfo}: {compInfo: CompInfo}) {
  const columns: GridColDef[] = [
    {
      field: 'filename',
      headerName: 'Filename',
      width: 350,
      editable: false,
    },
    {
      field: 'path',
      headerName: 'Path',
      width: 250,
      editable: false,
    },
    {
      field: 'size',
      headerName: 'Size',
      type: 'number',
      width: 100,
      editable: false,
      valueFormatter: (params: GridValueFormatterParams<number>) => {
        return humanFileSize(params.value)
      }
    },
  ];

  const rows = compInfo.left.map(ci => {
    return ({
      id: ci.path,
      filename: ci.filename,
      path: ci.path,
      size: ci.size
    })
  })

  return (
    <Container maxWidth="md">
      <DataGrid
        rows={rows}
        columns={columns}
        initialState={{
          pagination: {
            paginationModel: {
              pageSize: 10,
            },
          },
        }}
        pageSizeOptions={[10]}
        checkboxSelection
        // disableRowSelectionOnClick
      >

      </DataGrid>
    </Container>
  )
}

export default function Home() {
  const [dir1, setDir1] = useState<string>("")
  const [dir2, setDir2] = useState<string>("")
  const [compInfo, setCompInfo] = useState<CompInfo>()

  function handleGetDir(dialogTitle: string, setter: React.Dispatch<React.SetStateAction<string>>) {
    console.log("trying");
    
    invoke<string>("choose_folder", { dialogTitle: dialogTitle })
      .then(result => setter(result))
      .catch(console.error)
  }

  function handleFindDiffs() {
    invoke<any>("find_diffs", {dir1: dir1, dir2: dir2})
      .then(result => {
        console.log(result);
        setCompInfo(result)
      })
      .catch(console.error)
  }

  return (
    <main>
      <Container
        sx={{
          py: 3,
          textAlign: "center"
        }}
        maxWidth="lg"
      >
        <Box
          sx={{
            display: "flex",
            justifyContent: "space-around",
            mb: 3
          }}
        >
            <Paper>
              <Tooltip title={dir1}>
                <Box sx={{textAlign: "center"}}>
                  <Button fullWidth onClick={() => handleGetDir("Set Directory 1", setDir1)}>Set Directory 1</Button>
                  <Divider />
                  <Typography sx={{p: 1}}>{dir1.split(/[\/\\]/).slice(-1)}</Typography>
                </Box>
              </Tooltip>
            </Paper>
          <Paper>
            <Tooltip title={dir2}>
              <Box sx={{textAlign: "center"}}>
                <Button fullWidth onClick={() => handleGetDir("Set Directory 2", setDir2)}>Set Directory 2</Button>
                <Divider />
                <Typography sx={{p: 1}}>{dir2.split(/[\/\\]/).slice(-1)}</Typography>
              </Box>
            </Tooltip>
          </Paper>
        </Box>
        <Button variant="outlined" onClick={handleFindDiffs} sx={{mb: 3}}>Compare Directories</Button>
        {compInfo && <CopyGrid compInfo={compInfo} />}
      </Container>
    </main>
  );
}
