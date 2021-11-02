# Galeed modifications

Forked at commit 73b84b4ef3796617d5a840b583cb291252ccd241 .  Updated to work with Ubuntu 18 and expose a lower level API for Galeed.  

# libmpk

Documentation
-------------
Please check our ATC'19 [paper](https://www.usenix.org/conference/atc19/presentation/park-soyeon)

Environment
-------------
Tested on Ubuntu 16.04

Build
-------------

Build and install the kernel:

      - libmpk was developed on Linux 4.14.2.
      - The Linux kernel source is available in ./kernel
      - Enable memory protection keys when compiling the kernel.
      - Build and install the kernel.
      
Build general library:

```
$ cd lib
$ ./install.sh
```

Build heap-related library:

```
$ cd lib/heap
$ ./install.sh
```

Contacts
----------------
- Soyeon Park <soyeon@gatech.edu>
- Sangho Lee <Sangho.Lee@microsoft.com>
- Wen Xu <wen.xu@gatech.edu>
- Hyungon Moon <hyungon@unist.ac.kr>
- Taesoo Kim <taesoo@gatech.edu>


