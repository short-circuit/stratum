---
title: F1 Editor Test
tags:
- project
- editor
custom_field: keep-me-verbatim
---

- F1 Editor Test
  .heading-level: 1
  .id: 1f399b72-6af2-446f-a6ac-083fce7abfb4
- Welcome to the F1 editor verification page. This references [[Alpha Project]] and mentions Beta Notes in plain text.
  .id: 2be77ac7-fd0c-4ce2-b54e-da4ace83b0d0
- Parent one
  .id: 01f0c530-2807-4806-b81f-7f7d307df0ba
- Child of parent
  .id: f0841b8a-c40a-46ec-acee-ef3564859427
- ship the f1 mission
  .marker: TODO
  .id: 28142d10-32e2-42f1-8fcc-119eb77a4137
- secondary task
  .marker: DOING
  .priority: B
  .id: 096545f1-b0d4-499a-a097-04e72f46a04a
- third task with marker
  .marker: DOING
  .id: 6d2e7e45-283f-4e43-8757-025359452f2f
- completed task test
  .marker: DONE
  .id: 78ff3f9a-48f8-4a6f-a830-4ea230af46fb
- ## Math
  .id: 71f1a2b3-0000-0000-0000-000000000001
- Inline $E = mc^2$ and display:
  .id: 71f1a2b3-0000-0000-0000-000000000002
- $$\\int_0^1 x^2 dx = \\frac{1}{3}$$
  .id: 71f1a2b3-0000-0000-0000-000000000003
- ## Diagram
  .id: 71f1a2b3-0000-0000-0000-000000000004
- ```mermaid
  graph TD
  A[Start] --> B{Decision}
  B -->|yes| C[Continue]
  B -->|no| D[Stop]
  ```
  .id: 71f1a2b3-0000-0000-0000-000000000005
- ## XSS Test
  .id: 71f1a2b3-0000-0000-0000-000000000006
- Some text with <script>alert('xss')</script> and a raw HTML tag <img src=x onerror=alert(1)>.
  .id: 71f1a2b3-0000-0000-0000-000000000007
- [[Alpha Project]]
  .id: 71f1a2b3-0000-0000-0000-000000000008
