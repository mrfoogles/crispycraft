Things the code needs to do:

- Generate meshes
    - Setup building_blocks
    - Basic chunks in building_blocks (not empty)
    - Transform greedy_mesh() output into a usable format
- Draw meshes
    - Setup winit (windowing library)
        - update() function
        - render Event setup (OS tells you when to render)
    - Setup shaders (vtx, frag)
    - Setup camera (bind group, transform calculation)
    - Setup pipeline (layouts, bind groups, settings)
    - render() function that draws the meshes: DONE!