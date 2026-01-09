# Rusty FOIL
The intent was to implement [FOIL](https://dl.acm.org/doi/10.1023/A:1022699322624).
There's an "e2e test" you can run called `test_bongard_foil` to see how it works.

FOIL (First Order Inductive Learner) is an Inductive Logic Programming (ILP) system
which learns horn clauses from data. 
The wikipedia page has a simple description of the [algorithm](https://en.wikipedia.org/wiki/First-order_inductive_learner#Algorithm)

The [Bongard problem](https://en.wikipedia.org/wiki/Bongard_problem)s is a set of problems 
that are a popular toy dataset for ILP tasks. I'm going to try to implement [TILDE](https://www.sciencedirect.com/science/article/pii/S0004370298000344) 
based on [this](https://github.com/joschout/tilde) python implementation in the `tilde` branch.
