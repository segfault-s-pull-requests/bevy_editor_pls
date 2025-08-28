//! Some thoughts on persistence and uid
//! the basic issue of persistence is that it is a secondary concern.
//! like replication, we do not want to fiddle with already complicated spawning code.
//! instead we'd like persistence to happen in the background.
//! 
//! Everything that has persistant state has identity beyond the run of the program
//! therefore it needs a persistant id.
//! then persistance is as simple as 
//! 1. register persistant components, and their save / load functions
//! 2. save and load by Component + Uid 

