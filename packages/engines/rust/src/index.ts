/* eslint-disable */
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { createRequire } from 'node:module'

// Get the directory of the current module
const __dirname = fileURLToPath(new URL('.', import.meta.url))
const require = createRequire(import.meta.url)

const { platform, arch } = process

let nativeBinding: any = null
let localFileExisted = false
let loadError: Error | null = null

function isMusl(): boolean {
  // For Node 10
  if (!(process as any).report || typeof (process as any).report.getReport !== 'function') {
    try {
      const lddPath = require('child_process').execSync('which ldd').toString().trim()
      return readFileSync(lddPath, 'utf8').includes('musl')
    } catch (e) {
      return true
    }
  } else {
    const { glibcVersionRuntime } = (process as any).report.getReport().header
    return !glibcVersionRuntime
  }
}

// Helper to resolve binary path - binaries are in package root, not dist
function getBinaryPath(binaryName: string): string {
  // When running from dist/, binaries are in parent directory
  return join(__dirname, '..', binaryName)
}

switch (platform) {
  case 'android':
    switch (arch) {
      case 'arm64':
        localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.android-arm64.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.android-arm64.node'))
          } else {
            nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-android-arm64')
          }
        } catch (e) {
          loadError = e as Error
        }
        break
      case 'arm':
        localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.android-arm-eabi.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.android-arm-eabi.node'))
          } else {
            nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-android-arm-eabi')
          }
        } catch (e) {
          loadError = e as Error
        }
        break
      default:
        throw new Error(`Unsupported architecture on Android ${arch}`)
    }
    break
  case 'win32':
    switch (arch) {
      case 'x64':
        localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.win32-x64-msvc.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.win32-x64-msvc.node'))
          } else {
            nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-win32-x64-msvc')
          }
        } catch (e) {
          loadError = e as Error
        }
        break
      case 'ia32':
        localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.win32-ia32-msvc.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.win32-ia32-msvc.node'))
          } else {
            nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-win32-ia32-msvc')
          }
        } catch (e) {
          loadError = e as Error
        }
        break
      case 'arm64':
        localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.win32-arm64-msvc.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.win32-arm64-msvc.node'))
          } else {
            nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-win32-arm64-msvc')
          }
        } catch (e) {
          loadError = e as Error
        }
        break
      default:
        throw new Error(`Unsupported architecture on Windows: ${arch}`)
    }
    break
  case 'darwin':
    localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.darwin-universal.node'))
    try {
      if (localFileExisted) {
        nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.darwin-universal.node'))
      } else {
        nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-darwin-universal')
      }
      break
    } catch {}
    switch (arch) {
      case 'x64':
        localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.darwin-x64.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.darwin-x64.node'))
          } else {
            nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-darwin-x64')
          }
        } catch (e) {
          loadError = e as Error
        }
        break
      case 'arm64':
        localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.darwin-arm64.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.darwin-arm64.node'))
          } else {
            nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-darwin-arm64')
          }
        } catch (e) {
          loadError = e as Error
        }
        break
      default:
        throw new Error(`Unsupported architecture on macOS: ${arch}`)
    }
    break
  case 'freebsd':
    if (arch !== 'x64') {
      throw new Error(`Unsupported architecture on FreeBSD: ${arch}`)
    }
    localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.freebsd-x64.node'))
    try {
      if (localFileExisted) {
        nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.freebsd-x64.node'))
      } else {
        nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-freebsd-x64')
      }
    } catch (e) {
      loadError = e as Error
    }
    break
  case 'linux':
    switch (arch) {
      case 'x64':
        if (isMusl()) {
          localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.linux-x64-musl.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.linux-x64-musl.node'))
            } else {
              nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-linux-x64-musl')
            }
          } catch (e) {
            loadError = e as Error
          }
        } else {
          localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.linux-x64-gnu.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.linux-x64-gnu.node'))
            } else {
              nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-linux-x64-gnu')
            }
          } catch (e) {
            loadError = e as Error
          }
        }
        break
      case 'arm64':
        if (isMusl()) {
          localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.linux-arm64-musl.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.linux-arm64-musl.node'))
            } else {
              nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-linux-arm64-musl')
            }
          } catch (e) {
            loadError = e as Error
          }
        } else {
          localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.linux-arm64-gnu.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.linux-arm64-gnu.node'))
            } else {
              nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-linux-arm64-gnu')
            }
          } catch (e) {
            loadError = e as Error
          }
        }
        break
      case 'arm':
        if (isMusl()) {
          localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.linux-arm-musleabihf.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.linux-arm-musleabihf.node'))
            } else {
              nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-linux-arm-musleabihf')
            }
          } catch (e) {
            loadError = e as Error
          }
        } else {
          localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.linux-arm-gnueabihf.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.linux-arm-gnueabihf.node'))
            } else {
              nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-linux-arm-gnueabihf')
            }
          } catch (e) {
            loadError = e as Error
          }
        }
        break
      case 'riscv64':
        if (isMusl()) {
          localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.linux-riscv64-musl.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.linux-riscv64-musl.node'))
            } else {
              nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-linux-riscv64-musl')
            }
          } catch (e) {
            loadError = e as Error
          }
        } else {
          localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.linux-riscv64-gnu.node'))
          try {
            if (localFileExisted) {
              nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.linux-riscv64-gnu.node'))
            } else {
              nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-linux-riscv64-gnu')
            }
          } catch (e) {
            loadError = e as Error
          }
        }
        break
      case 's390x':
        localFileExisted = existsSync(getBinaryPath('mdx-hybrid-engine-rust.linux-s390x-gnu.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require(getBinaryPath('mdx-hybrid-engine-rust.linux-s390x-gnu.node'))
          } else {
            nativeBinding = require('@jp-knj/mdx-hybrid-engine-rust-linux-s390x-gnu')
          }
        } catch (e) {
          loadError = e as Error
        }
        break
      default:
        throw new Error(`Unsupported architecture on Linux: ${arch}`)
    }
    break
  default:
    throw new Error(`Unsupported OS: ${platform}, architecture: ${arch}`)
}

if (!nativeBinding) {
  if (loadError) {
    throw loadError
  }
  throw new Error(`Failed to load native binding`)
}

// Export the native bindings
export const { compileSync, isAvailable } = nativeBinding

// Type definitions
export interface CompileOptions {
  development?: boolean
  jsx?: boolean
  jsxRuntime?: 'automatic' | 'classic'
  jsxImportSource?: string
  providerImportSource?: string
  [key: string]: any
}

export interface CompileResult {
  code: string
  map?: any
}