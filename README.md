# xsched

## Sources

* jack
  * midi
  * transport
  * audio (eventually)
* osc


## OSC Namespace

All prefixed with `/xsched`

### Bindings

`/bindings/aliases`
    `/<name>` -> uuid
`/bindings/uuids/`
    `/<uuid>`
        `/type` -> `name`
        `/params`
            `/<name>` -> `uuid`

#### Example

```
/xsched/bindings/aliases/bpm -> "b907d0ee-8bea-4137-8a8e-2d49eff97b3a"
/xsched/bindings/aliases/period -> "96e3e763-23e7-49f6-b6d6-323d872f2383"

/xsched/bindings/uuids/80a1aedf-3e6d-4275-b197-d88c26299966/type -> "clock_data" "clock_data" "get"
/xsched/bindings/uuids/80a1aedf-3e6d-4275-b197-d88c26299966/params/bpm/type -> "value" "f64" "getset"
/xsched/bindings/uuids/80a1aedf-3e6d-4275-b197-d88c26299966/params/bpm/id -> "b907d0ee-8bea-4137-8a8e-2d49eff97b3a"
/xsched/bindings/uuids/80a1aedf-3e6d-4275-b197-d88c26299966/params/period/type -> "value" "f64" "getset"
/xsched/bindings/uuids/80a1aedf-3e6d-4275-b197-d88c26299966/params/period/id -> "96e3e763-23e7-49f6-b6d6-323d872f2383"
/xsched/bindings/uuids/80a1aedf-3e6d-4275-b197-d88c26299966/params/ppq/type -> "value" "usize" "getset"
/xsched/bindings/uuids/80a1aedf-3e6d-4275-b197-d88c26299966/params/ppq/id -> "f0e5e6b1-956f-4b63-8feb-109bb074bf77"

/xsched/bindings/uuids/3b361f55-6f3c-48b3-9f33-0035ec10a9cb/type -> "add" "f64" "get"
/xsched/bindings/uuids/3b361f55-6f3c-48b3-9f33-0035ec10a9cb/params/left/type -> "value" "f64" "get"
/xsched/bindings/uuids/3b361f55-6f3c-48b3-9f33-0035ec10a9cb/params/left/id -> nil
/xsched/bindings/uuids/3b361f55-6f3c-48b3-9f33-0035ec10a9cb/params/right/type -> "value" "f64" "get"
/xsched/bindings/uuids/3b361f55-6f3c-48b3-9f33-0035ec10a9cb/params/right/id -> nil

/xsched/bindings/uuids/7410338e-b799-4616-9418-3ef6d132007e/type -> "cast" "f64" "get"
/xsched/bindings/uuids/7410338e-b799-4616-9418-3ef6d132007e/params/in/id -> nil
/xsched/bindings/uuids/7410338e-b799-4616-9418-3ef6d132007e/params/in/type -> "value" "i64" "get"

/xsched/bindings/uuids/b907d0ee-8bea-4137-8a8e-2d49eff97b3a/type -> "value" "f64" "getset"
/xsched/bindings/uuids/b907d0ee-8bea-4137-8a8e-2d49eff97b3a/value -> 131.0
/xsched/bindings/uuids/96e3e763-23e7-49f6-b6d6-323d872f2383/type -> "value" "f64" 

```

`/xsched/bindings/available ->`
```json
{
  adds: {
    description: "saturating add of two values",
    variants: [
      {
        name: "f64",
        access: "get",
        type: "f64",
        params: {
          left: {
            type: "f64",
            access: "get"
          },
          right: {
            type: "f64",
            access: "get"
          }
        }
      },
      {
        name: "isize",
        access: "get",
        type: "isize",
        params: {
          left: {
            type: "isize",
            access: "get"
          },
          right: {
            type: "isize",
            access: "get"
          }
        }
      },
      {
        name: "usize",
        access: "get",
        type: "usize",
        params: {
          left: {
            type: "usize",
            access: "get"
          },
          right: {
            type: "usize",
            access: "get"
          }
        }
      },
      {
        name: "u8",
        access: "get",
        type: "u8",
        params: {
          left: {
            type: "u8",
            access: "get"
          },
          right: {
            type: "u8",
            access: "get"
          }
        }
      }
    ]
  }
}
```
