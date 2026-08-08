# Support Info Metadata Format Specification

## Overview

The Support Info metadata format describes the support lifecycle of RPM packages within one or more
DNF repositories. It provides a structured, machine-readable way to define support phases, timelines,
and classifications for packages. This allows users to map installed packages to their current and
future support levels, and allows OS vendors to deprecate subsets of the distribution on independent
schedules.

The format is consumed by the `dnf-plugin-support-info` DNF plugin, which downloads the XML from a
configured URL, caches it locally, and provides CLI commands for querying package support status.

## Discovery

In its current implementation, the DNF plugin does not expect "supportinfo" metadata to be
referenced in `repomd.xml` (unlike `primary.xml`, `filelists.xml`, etc.). Instead, the plugin is
configured with a direct URL via an INI-style configuration file (typically
`/etc/dnf/plugins/supportinfo.conf`). The XML file is fetched via HTTP(S) and may optionally be
accompanied by a detached GPG signature (`<url>.asc`).

A future implementation, should this be adopted mainstream, would likely expect it to be part
of the standard repository metadata set referenced in `repomd.xml`.

### Format Detection

A consumer determines the format by inspecting the root `<package_support>` element:

- If `schema_version="1.0"` is present, the document uses the **V1.0** format.
- If the `schema_version` attribute is absent, the document uses the **Legacy** format.

The legacy format is not supported by this library.

---

# V1.0 Format

## Hierarchy summary

```text
supportinfo.xml (V1.0)
└── <package_support> ──── schema_version="1.0", current_as
    ├── <lifecycles>
    │   └── <lifecycle>* ──── name, note?, display_name?, description?
    │       └── <phase>* ──── name, support_level, (start_date | start_milestone), display_name?
    ├── <support_milestones>
    │   └── <milestone>* ──── name, date, display_name?, description?
    ├── <support_levels>
    │   └── <support_level>* ──── name, severities, description?, display_name?
    ├── <packages>
    │   └── <package>* ──── name, lifecycle, origin, package_class?
    ├── <package_classes>
    │   └── <package_class>* ──── name
    │       ├── <summary>
    │       └── <text>
    ├── <package_origins>
    │   └── <package_origin>* ──── name, repo_id, dist, vendor, signing_key?, display_name?, description?
    └── <notes>
        └── <note>* ──── name
```

**Legend:**

- `*` = zero or more
- `?` = optional
- `(a | b)` = exactly one of a or b required

**Key constraints:**

- All container elements (`<lifecycles>`, `<support_milestones>`, etc.) are required
- Each `<lifecycle>` must have at least one `<phase>`
- Each `<support_level>` must have at least one entry (but zero milestones allowed)
- The pair `(name, origin)` must be unique across all `<package>` elements
- All references must be valid (lifecycles, milestones, support_levels, package_classes, package_origins, notes)

## Root Element: `<package_support>`

```xml
<package_support schema_version="1.0" current_as="2024-01-17T00:00:00">
  ...
</package_support>
```

| Attribute | Required | Type | Description |
|---|---|---|---|
| `schema_version` | Yes | `xs:string` | Must be `"1.0"`. Identifies the format version. |
| `current_as` | Yes | `xs:dateTime` | ISO 8601 timestamp indicating when the support data was last updated. |

### Child Elements (in order)

The root element contains exactly the following child elements, in this order:

1. `<lifecycles>` (required)
2. `<support_milestones>` (required)
3. `<support_levels>` (required)
4. `<packages>` (required)
5. `<package_classes>` (required)
6. `<package_origins>` (required)
7. `<notes>` (required)

All container elements are required to be present, though some may contain zero child elements
(see cardinality below).

---

## `<lifecycles>`

Container for one or more `<lifecycle>` elements. At least one `<lifecycle>` must be present.

### `<lifecycle>`

Defines a named support timeline as a sequence of phases. Packages reference a lifecycle by its
`name` attribute; this determines their entire support history and schedule.

```xml
<lifecycle name="eol_php81_lc" note="eol_php81"
           display_name="Php8.1 Lifecycle"
           description="Support timeline for php8.1 packages">
  <phase name="supported" support_level="standard" start_date="2023-03-15"/>
  <phase name="unsupported" support_level="eos" start_date="2026-11-25"/>
</lifecycle>
```

| Attribute | Required | Type | Description |
|---|---|---|---|
| `name` | Yes | `xs:string` | Unique identifier. Referenced by `<package lifecycle="...">`. |
| `note` | No | `xs:string` | References a `<note name="...">`. Provides supplemental context. |
| `display_name` | No | `xs:string` | Human-readable name (e.g. `"Php8.1 Lifecycle"`). |
| `description` | No | `xs:string` | Longer prose description. |

**Note:** The `display_name` and `description` attributes are used by implementations (e.g. the
legacy converter) but are not declared in the XSD schema. Consumers should accept their presence
without error but must not require them.

A `<lifecycle>` must contain at least one `<phase>` child element.

### `<phase>`

Defines a period within a lifecycle. Phases have only a start boundary; each phase continues until
the start of the next phase, or indefinitely if it is the last phase in the lifecycle.

```xml
<phase name="supported" support_level="standard" start_milestone="al2023_ga"/>
<phase name="unsupported" support_level="eos" start_date="2028-03-15"/>
```

| Attribute | Required | Type | Description |
|---|---|---|---|
| `name` | Yes | `xs:string` | Phase identifier. Canonical values are `"supported"` and `"unsupported"`, but any string is permitted. |
| `support_level` | Yes | `xs:string` | References a `<support_level name="...">`. |
| `start_date` | Conditional | `xs:date` | ISO 8601 date (YYYY-MM-DD) when this phase starts. Exactly one of `start_date` or `start_milestone` must be present. |
| `start_milestone` | Conditional | `xs:string` | References a `<milestone name="...">` whose `date` attribute is used as the start date. Exactly one of `start_date` or `start_milestone` must be present. |
| `display_name` | No | `xs:string` | Human-readable label. Not declared in the XSD but used by the plugin for display output. |

**Constraint (XSD 1.1 assert):**
```
(@start_date or @start_milestone) and not(@start_date and @start_milestone)
```
Exactly one of `start_date` or `start_milestone` must be specified, never both, never neither.

---

## `<support_milestones>`

Container for zero or more `<milestone>` elements. Milestones are named dates that can be reused
across multiple phases to avoid duplication of significant dates.

### `<milestone>`

```xml
<milestone name="al2023_ga" date="2023-03-15"
           display_name="AL2023 GA" description="General availability"/>
```

| Attribute | Required | Type | Description |
|---|---|---|---|
| `name` | Yes | `xs:string` | Unique identifier. Referenced by `<phase start_milestone="...">`. |
| `date` | Yes | `xs:date` | ISO 8601 date this milestone represents. |
| `display_name` | No | `xs:string` | Human-readable label. Not declared in the XSD. |
| `description` | No | `xs:string` | Additional descriptive text. Not declared in the XSD. |

---

## `<support_levels>`

Container for one or more `<support_level>` elements.

### `<support_level>`

Defines the scope of support offered during a phase by enumerating the severity classes that will
receive fixes.

```xml
<support_level name="standard" severities="Low,Medium,Important,Critical"
               description="Full security and bug fix support for all severities"/>
<support_level name="eos" severities=""
               description="End of support - no further updates"/>
```

| Attribute | Required | Type | Description |
|---|---|---|---|
| `name` | Yes | `xs:string` | Unique identifier. Referenced by `<phase support_level="...">`. |
| `severities` | Yes | `xs:string` | Comma-separated list of severity names that will be addressed. An empty string `""` means no patches (end-of-support). |
| `description` | No | `xs:string` | Human-readable description. Not declared in the XSD but used by the plugin for display output. |
| `display_name` | No | `xs:string` | Human-readable label. Not declared in the XSD but used by the plugin for display output. |

---

## `<packages>`

Container for zero or more `<package>` elements. This is a flat list; each package references its
lifecycle, origin, and optionally a class by name.

### `<package>`

```xml
<package name="test-glibc" lifecycle="eol_default_lc"
         package_class="core" origin="amazonlinux"/>
```

| Attribute | Required | Type | Description |
|---|---|---|---|
| `name` | Yes | `xs:string` | Source RPM package name (without version or architecture). |
| `lifecycle` | Yes | `xs:string` | References a `<lifecycle name="...">`. |
| `origin` | Yes | `xs:string` | References a `<package_origin name="...">`. |
| `package_class` | No | `xs:string` | References a `<package_class name="...">`. |

**Uniqueness constraint:** The pair `(name, origin)` must be unique across all `<package>` elements.
The same package name may appear more than once if originating from different origins.

---

## `<package_classes>`

Container for zero or more `<package_class>` elements. Classifies packages by intended use.

### `<package_class>`

```xml
<package_class name="core">
  <summary>Core system packages</summary>
  <text>Essential packages for system operation</text>
</package_class>
```

| Attribute/Element | Required | Type | Description |
|---|---|---|---|
| `name` (attribute) | Yes | `xs:string` | Unique identifier. Referenced by `<package package_class="...">`. |
| `<summary>` (child element) | Yes | `xs:string` | Short human-readable description. |
| `<text>` (child element) | Yes | `xs:string` | Longer detailed description. |

---

## `<package_origins>`

Container for one or more `<package_origin>` elements.

### `<package_origin>`

Defines the source repository and distribution information for packages.

```xml
<package_origin name="amazonlinux" repo_id="amazonlinux"
                dist="amzn2023" vendor="Amazon" signing_key="..."/>
```

| Attribute | Required | Type | Description |
|---|---|---|---|
| `name` | Yes | `xs:string` | Unique identifier. Referenced by `<package origin="...">`. |
| `repo_id` | Yes | `xs:string` | DNF repository ID (matches the `[repo_id]` stanza in a `.repo` file). |
| `dist` | Yes | `xs:string` | Distribution identifier, e.g. `"amzn2023"`. |
| `vendor` | Yes | `xs:string` | Vendor/publisher name, e.g. `"Amazon"`. |
| `signing_key` | No | `xs:string` | GPG signing key reference. |
| `display_name` | No | `xs:string` | Human-readable label. Not declared in the XSD but used by the plugin for display output. |
| `description` | No | `xs:string` | Human-readable description. Not declared in the XSD. |

---

## `<notes>`

Container for zero or more `<note>` elements.

### `<note>`

Provides supplemental text referenced by `<lifecycle>` elements via the `note` attribute.

```xml
<note name="eol_php81">Upstream end-of-life for PHP 8.1 is 2026-11-25</note>
```

| Attribute | Required | Type | Description |
|---|---|---|---|
| `name` | Yes | `xs:string` | Unique identifier. Referenced by `<lifecycle note="...">`. |
| *(text content)* | | `xs:string` | Free-form explanatory text. |

---

## Referential Integrity Constraints (V1.0)

The XSD defines the following key/keyref constraints:

| Constraint | Source | Target |
|---|---|---|
| `lifecycleKey` / `packageLifecycleRef` | `packages/package/@lifecycle` | `lifecycles/lifecycle/@name` |
| `milestoneKey` / `phaseMilestoneRef` | `lifecycles/lifecycle/phase/@start_milestone` | `support_milestones/milestone/@name` |
| `supportLevelKey` / `phaseSupportLevelRef` | `lifecycles/lifecycle/phase/@support_level` | `support_levels/support_level/@name` |
| `packageClassKey` / `packagePackageClassRef` | `packages/package/@package_class` | `package_classes/package_class/@name` |
| `packageOriginKey` / `packagePackageOriginRef` | `packages/package/@origin` | `package_origins/package_origin/@name` |
| `noteKey` / `lifecycleNoteRef` | `lifecycles/lifecycle/@note` | `notes/note/@name` |
| `uniquePackageNameOrigin` | `packages/package/(@name, @origin)` | *(unique)* |

---

## Complete V1.0 Example

```xml
<?xml version="1.0" ?>
<package_support schema_version="1.0" current_as="2024-01-17T00:00:00">
  <lifecycles>
    <lifecycle name="eol_default_lc" display_name="Al2023 Lifecycle"
               description="Support timeline for al2023 packages">
      <phase name="supported" support_level="standard" start_milestone="al2023_ga"/>
      <phase name="unsupported" support_level="eos" start_milestone="al2023_eol"/>
    </lifecycle>
    <lifecycle name="eol_php81_lc" note="eol_php81" display_name="Php8.1 Lifecycle"
               description="Support timeline for php8.1 packages">
      <phase name="supported" support_level="standard" start_date="2023-03-15"/>
      <phase name="unsupported" support_level="eos" start_date="2026-11-25"/>
    </lifecycle>
    <lifecycle name="eol_python36_lc" note="eol_python36" display_name="Python3.6 Lifecycle"
               description="Support timeline for python3.6 packages">
      <phase name="unsupported" support_level="eos" start_date="2023-03-15"/>
    </lifecycle>
  </lifecycles>
  <support_milestones>
    <milestone name="al2023_ga" date="2023-03-15"
               display_name="AL2023 GA" description="General availability"/>
    <milestone name="al2023_eol" date="2028-03-15"
               display_name="AL2023 EOL" description="End of life"/>
  </support_milestones>
  <support_levels>
    <support_level name="standard" severities="Low,Medium,Important,Critical"
                   description="Full security and bug fix support for all severities"/>
    <support_level name="eos" severities=""
                   description="End of support - no further updates"/>
  </support_levels>
  <packages>
    <package name="test-bash" lifecycle="eol_default_lc"
             package_class="core" origin="amazonlinux"/>
    <package name="test-glibc" lifecycle="eol_default_lc"
             package_class="core" origin="amazonlinux"/>
    <package name="test-php81" lifecycle="eol_php81_lc"
             package_class="core" origin="amazonlinux"/>
    <package name="test-python36" lifecycle="eol_python36_lc"
             package_class="core" origin="amazonlinux"/>
  </packages>
  <package_classes>
    <package_class name="core">
      <summary>Core system packages</summary>
      <text>Essential packages for system operation</text>
    </package_class>
  </package_classes>
  <package_origins>
    <package_origin name="amazonlinux" repo_id="amazonlinux"
                    dist="amzn2023" vendor="Amazon"/>
  </package_origins>
  <notes>
    <note name="eol_php81">Upstream end-of-life for PHP 8.1 is 2026-11-25</note>
    <note name="eol_python36">Python 3.6 reached upstream end-of-life on 2021-12-23</note>
  </notes>
</package_support>
```

---

## Attributes Not Declared in the XSD

The v1.0 XSD schema does not declare every attribute that appears in production XML. The
`dnf-plugin-support-info` plugin's handler library (`supportinfo.handler`) defines dataclass fields
for these attributes, and the plugin actively uses several of them for v1 display output — they are
not solely legacy artifacts. Because the XSD uses `lax` processing, parsers that validate against
it will accept these extra attributes without error.

The following attributes are present in production data (e.g. the AL2023 support info XML) and used
by the plugin, but absent from the XSD:

| Element | Attribute | Plugin usage |
| --- | --- | --- |
| `<lifecycle>` | `display_name`, `description` | `display_name` used by the legacy converter for summary generation; `description` used by the legacy converter |
| `<milestone>` | `display_name`, `description` | Not directly consumed by the plugin, but accepted and stored by the handler |
| `<phase>` | `display_name` | Preferred over `name` for current phase display output |
| `<support_level>` | `display_name` | Preferred over `name` for support level and patch priority display |
| `<support_level>` | `description` | Shown in `--pkg` output and `--list-filters` output |
| `<package_origin>` | `display_name` | Preferred over `name` for origin display output |
| `<package_origin>` | `description` | Present in production data but not consumed by the plugin |

Consumers should accept these attributes when present and fall back to `name` when `display_name`
is absent.
