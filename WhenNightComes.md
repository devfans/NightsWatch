#### When night comes, we start to watch with a brave heart and a clear mind:

We would want the design simple enough but still powerful. No one wants to spend much on learning how to use a monitoring tool. So, just two roles: the server and the agent.

## What does a server do?
+ Listen for agents to receive data from agents
+ Calculate serivce heath stats
+ Send alerts to the army to protect the wall
+ Draw service health map
+ Pipe metric data into other time serie data store like grahite (Optional)

## What does a agent do?
+ Load the watch targets from the configuration file
+ Follow the instruction to stare at targets, or which an interval for nap time.
+ Report the data into the server


