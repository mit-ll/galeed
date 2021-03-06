# Rust Bibliography

This is a reading list of material relevant to Rust. It includes prior
research that has - at one time or another - influenced the design of
Rust, as well as publications about Rust.

## Type system

* [Region based memory management in Cyclone](https://www.cs.umd.edu/projects/cyclone/papers/cyclone-regions.pdf)
* [Safe manual memory management in Cyclone](http://www.cs.umd.edu/projects/PL/cyclone/scp.pdf)
* [Making ad-hoc polymorphism less ad hoc](https://dl.acm.org/doi/10.1145/75277.75283)
* [Macros that work together](https://www.cs.utah.edu/plt/publications/jfp12-draft-fcdf.pdf)
* [Traits: composable units of behavior](http://scg.unibe.ch/archive/papers/Scha03aTraits.pdf)
* [Alias burying](http://www.cs.uwm.edu/faculty/boyland/papers/unique-preprint.ps) - We tried something similar and abandoned it.
* [External uniqueness is unique enough](http://www.cs.uu.nl/research/techreps/UU-CS-2002-048.html)
* [Uniqueness and Reference Immutability for Safe Parallelism](https://research.microsoft.com/pubs/170528/msr-tr-2012-79.pdf)
* [Region Based Memory Management](http://www.cs.ucla.edu/~palsberg/tba/papers/tofte-talpin-iandc97.pdf)

## Concurrency

* [Singularity: rethinking the software stack](https://research.microsoft.com/pubs/69431/osr2007_rethinkingsoftwarestack.pdf)
* [Language support for fast and reliable message passing in singularity OS](https://research.microsoft.com/pubs/67482/singsharp.pdf)
* [Scheduling multithreaded computations by work stealing](http://supertech.csail.mit.edu/papers/steal.pdf)
* [Thread scheduling for multiprogramming multiprocessors](http://www.eecis.udel.edu/%7Ecavazos/cisc879-spring2008/papers/arora98thread.pdf)
* [The data locality of work stealing](http://www.aladdin.cs.cmu.edu/papers/pdfs/y2000/locality_spaa00.pdf)
* [Dynamic circular work stealing deque](http://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.170.1097&rep=rep1&type=pdf) - The Chase/Lev deque
* [Work-first and help-first scheduling policies for async-finish task parallelism](http://www.cs.rice.edu/%7Eyguo/pubs/PID824943.pdf) - More general than fully-strict work stealing
* [A Java fork/join calamity](https://web.archive.org/web/20190904045322/http://www.coopsoft.com/ar/CalamityArticle.html) - critique of Java's fork/join library, particularly its application of work stealing to non-strict computation
* [Scheduling techniques for concurrent systems](http://www.stanford.edu/~ouster/cgi-bin/papers/coscheduling.pdf)
* [Contention aware scheduling](http://www.blagodurov.net/files/a8-blagodurov.pdf)
* [Balanced work stealing for time-sharing multicores](https://web.njit.edu/~dingxn/papers/BWS.pdf)
* [Three layer cake for shared-memory programming](http://dl.acm.org/citation.cfm?id=1953616&dl=ACM&coll=DL&CFID=524387192&CFTOKEN=44362705)
* [Non-blocking steal-half work queues](http://www.cs.bgu.ac.il/%7Ehendlerd/papers/p280-hendler.pdf)
* [Reagents: expressing and composing fine-grained concurrency](https://aturon.github.io/academic/reagents.pdf)
* [Algorithms for scalable synchronization of shared-memory multiprocessors](https://www.cs.rochester.edu/u/scott/papers/1991_TOCS_synch.pdf)
* [Epoch-based reclamation](https://www.cl.cam.ac.uk/techreports/UCAM-CL-TR-579.pdf).

## Others

* [Crash-only software](https://www.usenix.org/legacy/events/hotos03/tech/full_papers/candea/candea.pdf)
* [Composing High-Performance Memory Allocators](http://people.cs.umass.edu/~emery/pubs/berger-pldi2001.pdf)
* [Reconsidering Custom Memory Allocation](http://people.cs.umass.edu/~emery/pubs/berger-oopsla2002.pdf)

## Papers *about* Rust

* [GPU Programming in Rust: Implementing High Level Abstractions in a Systems
  Level
  Language](https://ieeexplore.ieee.org/document/6650903).
  Early GPU work by Eric Holk.
* [Parallel closures: a new twist on an old
  idea](https://www.usenix.org/conference/hotpar12/parallel-closures-new-twist-old-idea)
  - not exactly about Rust, but by nmatsakis
* [Patina: A Formalization of the Rust Programming
  Language](http://dada.cs.washington.edu/research/tr/2015/03/UW-CSE-15-03-02.pdf).
  Early formalization of a subset of the type system, by Eric Reed.
* [Experience Report: Developing the Servo Web Browser Engine using
  Rust](http://arxiv.org/abs/1505.07383). By Lars Bergstrom.
* [Implementing a Generic Radix Trie in
  Rust](https://michaelsproul.github.io/rust_radix_paper/rust-radix-sproul.pdf). Undergrad
  paper by Michael Sproul.
* [Reenix: Implementing a Unix-Like Operating System in
  Rust](https://scialex.github.io/reenix.pdf). Undergrad paper by Alex
  Light.
* [Evaluation of performance and productivity metrics of potential programming languages in the HPC environment](https://github.com/1wilkens/thesis-ba).
  Bachelor's thesis by Florian Wilkens. Compares C, Go and Rust.
* [Nom, a byte oriented, streaming, zero copy, parser combinators library
  in Rust](http://spw15.langsec.org/papers/couprie-nom.pdf). By
  Geoffroy Couprie, research for VLC.
* [Graph-Based Higher-Order Intermediate
  Representation](http://compilers.cs.uni-saarland.de/papers/lkh15_cgo.pdf). An
  experimental IR implemented in Impala, a Rust-like language.
* [Code Refinement of Stencil
  Codes](http://compilers.cs.uni-saarland.de/papers/ppl14_web.pdf). Another
  paper using Impala.
* [Parallelization in Rust with fork-join and
  friends](http://publications.lib.chalmers.se/records/fulltext/219016/219016.pdf). Linus
  Farnstrand's master's thesis.
* [Session Types for
  Rust](http://munksgaard.me/papers/laumann-munksgaard-larsen.pdf). Philip
  Munksgaard's master's thesis. Research for Servo.
* [Ownership is Theft: Experiences Building an Embedded OS in Rust - Amit Levy, et. al.](http://amitlevy.com/papers/tock-plos2015.pdf)
* [You can't spell trust without Rust](https://raw.githubusercontent.com/Gankro/thesis/master/thesis.pdf). Alexis Beingessner's master's thesis.
* [Rust-Bio: a fast and safe bioinformatics library](http://bioinformatics.oxfordjournals.org/content/early/2015/10/06/bioinformatics.btv573). Johannes K??ster
* [Safe, Correct, and Fast Low-Level Networking](http://citeseerx.ist.psu.edu/viewdoc/summary?doi=10.1.1.704.1768). Robert Clipsham's master's thesis.
* [Formalizing Rust traits](http://hdl.handle.net/2429/55609). Jonatan Milewski's master's thesis.
* [Rust as a Language for High Performance GC Implementation](http://users.cecs.anu.edu.au/~steveb/downloads/pdf/rust-ismm-2016.pdf)
* [Simple Verification of Rust Programs via Functional Purification](https://github.com/Kha/electrolysis). Sebastian Ullrich's master's thesis.
* [Writing parsers like it is 2017](http://spw17.langsec.org/papers/chifflier-parsing-in-2017.pdf) Pierre Chifflier and Geoffroy Couprie for the Langsec Workshop
* [The Case for Writing a Kernel in Rust](https://www.tockos.org/assets/papers/rust-kernel-apsys2017.pdf)
* [RustBelt: Securing the Foundations of the Rust Programming Language](https://plv.mpi-sws.org/rustbelt/popl18/)
