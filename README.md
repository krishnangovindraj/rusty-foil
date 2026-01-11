# Rusty FOIL
The intent was to implement [FOIL](https://dl.acm.org/doi/10.1023/A:1022699322624) using TypeQL as representation - 
FOIL is a "First Order Inductive Learner" is an Inductive Logic Programming (ILP) system which learns horn clauses from data. 
The wikipedia page has a simple description of the [algorithm](https://en.wikipedia.org/wiki/First-order_inductive_learner#Algorithm).
There's an "e2e test" you can run called `test_bongard_foil` to see how it works (assuming I got it right).

I also tried to implement [TILDE](https://dtai-static.cs.kuleuven.be/publications/files/2580.pdf), a first-order decision tree learner. 
Although I've only implemented a minimal POC which does classification, TILDE can do regression trees, clustering, 
and possibly anomaly detection using isolation forests (this was what I did for my master's thesis, extending [joschout/tilde](https://github.com/joschout/tilde)) 
There's an "e2e test" `test_bongard_tilde`.

## State
I've implemented some naive "refinement" operators for TypeQL - i.e. I add one more constraint to a given TypeQL query.
The hypothesis language is "discovered" from the schema of the TypeDB database.
The current refinements are:
1. adding `isa` constraint to refine the type of a variable
2. adding `links` constraint from a player to a relation, with a role specified.
3. adding `links` constraint from a relation to a player, with a role specified
4. 2 followed by 3 in a single step (because that's more informative)
5. adding a `has` constraint which specifies attribute type AND value.

## Incomplete & nice future work:
* Just adding attributes, though the approach of adding this and then constraining it with a value also makes sense.
* Unifying variables
* Comparing attribute variables, or attributes to constants.
* Add negation to FOIL (I think it's implicit in TILDE's right children)
* Make FOIL recursive.
* Other algorithms

## Why I like ILP
ILP is pretty simple, but pretty cool. A problem is defined by the hypothesis language (here, TypeQL), positive examples, negative examples, and possibly background knowledge (e.g. functions). The output is one or more clauses (TypeQL queries) which satisfy the positive examples but not the negative ones.
An interesting early task was trying to learn what made molecules mutagenic based on their structure encoded in prolog.

I'm mostly familiar with top-down ILP (such as foil), where you start with the empty clause and add literals 
till you cover positive examples and not the negative ones. 
These are excellent at learning determistic concepts, but not not great at real-world noisy data.
How well they work depends on the hypothesis language, since we can only learn concepts that it can express.
Unsurprisingly, these blow up quite fast with the size of the language. 
There are clever techniques to combat this, including bottom-up approaches based on LGGs and inverted resolution exist too (cigol?), or bottom-clause guided top-down search (aleph?).

What's great about top-down rule-learning approaches is that you can learn recursive concepts, such as transitive reachability (or quicksort).
Newer approaches of ILP, called meta-interpretive learning also use "meta-rules" to "invent predicates" they need to learn the concept (metagol).
Meta-interpretive because they generate programs for a prolog meta-interpreter. Presumably, this can learn `partition` based on more primitive predicates.
As you'd imagine, these programs can be expensive to learn. My friend worked on [guiding the search using neural networks](https://arxiv.org/abs/1804.01186) the search with some interesting results. Today, LLMs can write the program themselves and refine based on tests, so these may not be so relevant.
What interests me but I haven't looked much into is program synthesis which "*[construct a program that provably satisfies a given high-level formal specification](https://en.wikipedia.org/wiki/Program_synthesis)*".

I'm getting carried away, because TypeQL isn't a programming language like prolog. But TypeQL *is* a database, so data-mining tasks might work well.
TILDE (part of the ACE data-mining system) can do classification, clustering and anomaly-detection. 
WARMR, an association rule-mining system was used to detect [irregularities in road-data](https://dl.acm.org/doi/10.1016/j.eswa.2011.09.125)
which could mean bad data, or weird roads.

